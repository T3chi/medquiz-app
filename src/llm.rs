use serde::Deserialize;
use serde_json::Value;

use crate::domain::citations::{build_citation, find_verified_quote};
use crate::domain::explanations::options_have_explanations;
use crate::domain::nbme::{difficulty_guidance, validate_question, NBME_SYSTEM_RULES};
use crate::domain::subjects::normalize_subjects;
use crate::models::{AppSettings, Question, QuestionOption, QuizSettings};
use crate::parsers::truncate_text;

const MAX_REGENERATION_ATTEMPTS: u32 = 2;

pub struct QuestionGenerator {
    client: reqwest::Client,
    settings: AppSettings,
}

impl QuestionGenerator {
    pub fn new(settings: AppSettings) -> anyhow::Result<Self> {
        if settings.api_key.trim().is_empty() {
            anyhow::bail!("API key not configured. Go to Settings and add your OpenAI or LM Studio API key.");
        }
        Ok(Self {
            client: reqwest::Client::new(),
            settings,
        })
    }

    pub async fn generate(
        &self,
        source_text: &str,
        source_file_id: &str,
        quiz_settings: &QuizSettings,
        filename: &str,
        professor_name: Option<String>,
        exclude_stems: &[String],
        mut on_progress: impl FnMut(&str, f64),
    ) -> anyhow::Result<Vec<Question>> {
        let full_text = source_text;
        let text = truncate_text(source_text, 12000);
        let count = quiz_settings.question_count as usize;
        let mut all_questions = Vec::new();
        let mut remaining = count;
        let mut attempt = 0u32;
        let mut excluded: Vec<String> = exclude_stems.to_vec();

        while remaining > 0 && attempt <= MAX_REGENERATION_ATTEMPTS {
            if attempt > 0 {
                on_progress(
                    &format!("Retrying generation ({})...", attempt + 1),
                    all_questions.len() as f64 / count as f64,
                );
            }
            on_progress("Calling AI model...", all_questions.len() as f64 / count as f64);
            let raw_batch = self
                .call_model(
                    &text,
                    quiz_settings,
                    remaining,
                    &all_questions,
                    filename,
                    professor_name.as_deref(),
                    &excluded,
                )
                .await?;
            on_progress("Validating questions...", all_questions.len() as f64 / count as f64);

            for raw in raw_batch {
                match self.normalize(
                    &raw,
                    source_file_id,
                    quiz_settings,
                    full_text,
                    filename,
                    professor_name.clone(),
                ) {
                    Ok(normalized) => {
                        if !validate_question(&normalized.stem, &normalized.options, &normalized.correct_answer)
                            .is_empty()
                        {
                            continue;
                        }
                        if normalized.citation.as_ref().and_then(|c| c.quote.as_ref()).is_none() {
                            continue;
                        }
                        excluded.push(normalized.stem.clone());
                        all_questions.push(normalized);
                        remaining = remaining.saturating_sub(1);
                        on_progress(
                            &format!("Generated {} of {count} questions", all_questions.len()),
                            all_questions.len() as f64 / count as f64,
                        );
                        if remaining == 0 {
                            break;
                        }
                    }
                    Err(_) => continue,
                }
            }
            attempt += 1;
        }

        if all_questions.len() < count {
            anyhow::bail!(
                "Could only generate {} of {count} NBME-compliant questions with valid citations.",
                all_questions.len()
            );
        }
        Ok(all_questions.into_iter().take(count).collect())
    }

    async fn call_model(
        &self,
        source_text: &str,
        settings: &QuizSettings,
        count: usize,
        existing: &[Question],
        filename: &str,
        professor_name: Option<&str>,
        exclude_stems: &[String],
    ) -> anyhow::Result<Vec<Value>> {
        let url = format!(
            "{}/chat/completions",
            self.settings.api_base_url.trim_end_matches('/')
        );
        let body = serde_json::json!({
            "model": self.settings.model,
            "temperature": 0.5,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": system_prompt(settings) },
                { "role": "user", "content": user_prompt(source_text, settings, count, existing, filename, professor_name, exclude_stems) }
            ]
        });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.settings.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No response from AI model"))?;
        let parsed: LlmResponse = serde_json::from_str(content)?;
        Ok(parsed.questions)
    }

    fn normalize(
        &self,
        raw: &Value,
        source_file_id: &str,
        settings: &QuizSettings,
        full_text: &str,
        filename: &str,
        professor_name: Option<String>,
    ) -> anyhow::Result<Question> {
        let vignette = raw["vignette"].as_str().unwrap_or("").trim();
        let lead_in = raw["leadIn"]
            .as_str()
            .or_else(|| raw["lead_in"].as_str())
            .unwrap_or("")
            .trim();
        let legacy = raw["stem"].as_str().unwrap_or("").trim();
        let stem = if !vignette.is_empty() && !lead_in.is_empty() {
            format!("{vignette}\n\n{lead_in}")
        } else if !legacy.is_empty() {
            legacy.to_string()
        } else {
            anyhow::bail!("missing stem");
        };

        let labels = ["A", "B", "C", "D", "E"];
        let raw_opts = raw["options"].as_array().ok_or_else(|| anyhow::anyhow!("missing options"))?;
        let mut options = Vec::new();
        for (i, opt) in raw_opts.iter().take(5).enumerate() {
            options.push(QuestionOption {
                label: labels[i].to_string(),
                text: opt["text"].as_str().unwrap_or("").trim().to_string(),
                explanation: opt["explanation"].as_str().unwrap_or("").trim().to_string(),
            });
        }
        if options.len() != 5 || !options_have_explanations(&options) {
            anyhow::bail!("invalid options");
        }

        let correct = raw["correctAnswer"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_uppercase();
        if !labels.contains(&correct.as_str()) {
            anyhow::bail!("invalid correct answer");
        }

        let raw_quote = raw["sourceQuote"]
            .as_str()
            .or_else(|| raw["source_quote"].as_str())
            .unwrap_or("")
            .trim();
        let verified = find_verified_quote(raw_quote, full_text).ok_or_else(|| anyhow::anyhow!("bad quote"))?;

        let cited_prof = raw["sourceProfessor"]
            .as_str()
            .or_else(|| raw["source_professor"].as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !matches!(s.to_lowercase().as_str(), "null" | "none" | "n/a"));
        let final_prof = cited_prof.or(professor_name);

        let mut subjects = normalize_subjects(&raw["subjects"]);
        if subjects.is_empty() {
            if let Some(topic) = raw["topic"].as_str() {
                subjects = normalize_subjects(&Value::String(topic.to_string()));
            }
        }

        Ok(Question {
            id: String::new(),
            source_file_id: source_file_id.to_string(),
            stem,
            options,
            correct_answer: correct,
            explanation: raw["explanation"].as_str().unwrap_or("").trim().to_string(),
            difficulty: settings.difficulty.clone(),
            exam_style: settings.exam_style.clone(),
            topic: raw["topic"].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            subjects,
            citation: Some(build_citation(filename, final_prof, verified)),
            last_result: None,
            last_answered_at: None,
            attempt_count: 0,
            created_at: None,
        })
    }
}

#[derive(Deserialize)]
struct LlmResponse {
    questions: Vec<Value>,
}

fn system_prompt(settings: &QuizSettings) -> String {
    let exam_note = if settings.exam_style == "COMLEX" {
        "COMLEX-SPECIFIC: Integrate osteopathic principles when supported by source.\n"
    } else {
        "USMLE-SPECIFIC: Mirror USMLE Step 1/Step 2 CK NBME style.\n"
    };
    format!(
        "{NBME_SYSTEM_RULES}\n{exam_note}\n\
        SOURCE CITATION: Include sourceQuote verbatim from source.\n\
        SUBJECT TAGGING: Include subjects array (1-3 lowercase tags).\n\
        OPTION EXPLANATIONS: Every option needs explanation field.\n\
        Respond ONLY with JSON: {{\"questions\":[{{\"vignette\",\"leadIn\",\"options\":[{{\"label\",\"text\",\"explanation\"}}],\"correctAnswer\",\"explanation\",\"topic\",\"subjects\",\"sourceQuote\",\"sourceProfessor\"}}]}}"
    )
}

fn user_prompt(
    source_text: &str,
    settings: &QuizSettings,
    count: usize,
    existing: &[Question],
    filename: &str,
    professor_name: Option<&str>,
    exclude_stems: &[String],
) -> String {
    let prof_line = professor_name
        .map(|p| format!("DETECTED PROFESSOR: {p}"))
        .unwrap_or_else(|| "PROFESSOR: search source text".into());
    let mut avoid: Vec<String> = exclude_stems.to_vec();
    for q in existing {
        let item = q
            .topic
            .clone()
            .unwrap_or_else(|| q.stem.chars().take(80).collect());
        avoid.push(item);
        if avoid.len() >= 30 {
            break;
        }
    }
    let avoid_block = if avoid.is_empty() {
        String::new()
    } else {
        format!("\nDO NOT REPEAT:\n{}", avoid.iter().map(|t| format!("- {t}")).collect::<Vec<_>>().join("\n"))
    };
    format!(
        "Write exactly {count} NBME items.\nFILE: {filename}\n{prof_line}\nEXAM: {}\nDIFFICULTY: {}\n{}\n\
         MANDATORY: vignette+leadIn, 5 options with explanations, subjects, sourceQuote.\n{avoid_block}\n\nSOURCE:\n---\n{source_text}\n---",
        settings.exam_style,
        settings.difficulty,
        difficulty_guidance(&settings.difficulty),
    )
}