use regex::Regex;
use serde_json::Value;

use crate::domain::citations::{build_citation, find_verified_quote};
use crate::domain::subjects::normalize_subjects;
use crate::models::{Citation, Question, SourceFileDetail};

const FILENAME_TAG_HINTS: &[(&str, &str)] = &[
    ("pain", "pain"),
    ("shock", "shock"),
    ("neoplas", "oncology"),
    ("metastas", "metastasis"),
    ("environmental", "environmental pathology"),
    ("fluid", "hemodynamics"),
    ("repair", "tissue repair"),
    ("wound", "wound healing"),
];

const TOPIC_STOPWORDS: &[&str] = &[
    "management", "mechanisms", "mechanism", "types", "stages", "spread",
];

pub fn infer_subjects(topic: Option<&str>, filename: &str) -> Vec<String> {
    let mut raw: Vec<String> = Vec::new();

    if let Some(topic) = topic.map(str::trim).filter(|t| !t.is_empty()) {
        let lower = topic.to_lowercase();
        raw.push(lower.clone());

        if let Some(idx) = lower.rfind(" of ") {
            let focus = lower[idx + 4..].trim();
            if focus.len() > 2 {
                raw.push(focus.to_string());
            }
        }

        for part in lower.split(['/', ',']) {
            let part = part.trim();
            if part.len() > 2 && part != lower {
                raw.push(part.to_string());
            }
        }
    }

    let filename_lower = filename.to_lowercase();
    for (hint, label) in FILENAME_TAG_HINTS {
        if filename_lower.contains(hint) {
            raw.push((*label).to_string());
        }
    }

    let mut tags = normalize_subjects(&Value::Array(
        raw.into_iter().map(Value::String).collect(),
    ));

    tags.retain(|t| !TOPIC_STOPWORDS.contains(&t.as_str()));
    if tags.len() > 3 {
        tags.truncate(3);
    }
    if tags.is_empty() {
        if let Some(topic) = topic.map(str::trim).filter(|t| !t.is_empty()) {
            tags = normalize_subjects(&Value::String(topic.to_lowercase()));
        }
    }
    tags
}

pub fn infer_citation(question: &Question, source: &SourceFileDetail) -> Option<Citation> {
    if question
        .citation
        .as_ref()
        .and_then(|c| c.quote.as_ref())
        .is_some_and(|q| !q.trim().is_empty())
    {
        return question.citation.clone();
    }

    let correct_text = question
        .options
        .iter()
        .find(|o| o.label == question.correct_answer)
        .map(|o| o.text.as_str())
        .unwrap_or("");

    for candidate in citation_candidates(&question.explanation, correct_text, question.topic.as_deref()) {
        if let Some(verified) = find_verified_quote(&candidate, &source.text_content) {
            if verified.len() >= 25 {
                return Some(build_citation(
                    &source.filename,
                    source.professor_name.clone(),
                    verified,
                ));
            }
        }
    }

    if let Some(topic) = question.topic.as_deref() {
        if let Some(span) = extract_topic_span(topic, &source.text_content) {
            if let Some(verified) = find_verified_quote(&span, &source.text_content) {
                return Some(build_citation(
                    &source.filename,
                    source.professor_name.clone(),
                    verified,
                ));
            }
            if span.len() >= 30 {
                return Some(build_citation(
                    &source.filename,
                    source.professor_name.clone(),
                    span,
                ));
            }
        }
    }

    None
}

pub fn enrich_citation(citation: &mut Citation, source: &SourceFileDetail) {
    if citation.filename.as_ref().is_none_or(|f| f.is_empty()) {
        citation.filename = Some(source.filename.clone());
    }
    if citation.professor_name.as_ref().is_none_or(|p| p.is_empty()) {
        citation.professor_name = source.professor_name.clone();
    }
}

fn citation_candidates(explanation: &str, correct_text: &str, topic: Option<&str>) -> Vec<String> {
    let mut candidates = Vec::new();

    if !explanation.trim().is_empty() {
        let sentence_re = Regex::new(r"(?m)[^.!?]+[.!?]").unwrap();
        for cap in sentence_re.find_iter(explanation) {
            let sent = cap.as_str().trim();
            if sent.len() >= 35 {
                candidates.push(sent.to_string());
            }
        }
        let words: Vec<_> = explanation.split_whitespace().collect();
        for size in [14_usize, 12, 10, 8] {
            if words.len() >= size {
                candidates.push(words[..size].join(" "));
            }
        }
    }

    if correct_text.len() >= 20 {
        candidates.push(correct_text.to_string());
    }

    if let Some(topic) = topic.map(str::trim).filter(|t| !t.is_empty()) {
        candidates.push(topic.to_string());
    }

    candidates.sort_by_key(|c| std::cmp::Reverse(c.len()));
    candidates
}

fn extract_topic_span(topic: &str, source_text: &str) -> Option<String> {
    let topic_lower = topic.to_lowercase();
    let needles: Vec<String> = if let Some(m) = find_case_insensitive(source_text, &topic_lower) {
        vec![m]
    } else {
        topic_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| w.to_string())
            .filter(|w| find_case_insensitive(source_text, w).is_some())
            .collect()
    };

    let needle = needles.into_iter().max_by_key(|n| n.len())?;
    let mat = find_match(source_text, &needle)?;
    let start = mat.start().saturating_sub(80);
    let end = (mat.end() + 180).min(source_text.len());
    let start = source_text
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= start)
        .unwrap_or(0);
    let end = source_text
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= end)
        .unwrap_or(source_text.len());
    let mut excerpt = source_text[start..end].trim().to_string();
    excerpt = collapse_whitespace(&excerpt);

    if excerpt.chars().count() > 280 {
        let truncated: String = excerpt.chars().take(280).collect();
        if let Some(pos) = truncated.rfind('.') {
            if pos > 60 {
                excerpt = truncated[..=pos].to_string();
            } else {
                excerpt = format!("{truncated}…");
            }
        } else {
            excerpt = format!("{truncated}…");
        }
    }

    if excerpt.len() >= 30 {
        Some(excerpt)
    } else {
        None
    }
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn find_case_insensitive<'a>(haystack: &'a str, needle: &str) -> Option<String> {
    find_match(haystack, needle).map(|m| m.as_str().to_string())
}

fn find_match<'a>(haystack: &'a str, needle: &str) -> Option<regex::Match<'a>> {
    if needle.is_empty() {
        return None;
    }
    let pattern = format!(r"(?i){}", regex::escape(needle));
    Regex::new(&pattern).ok()?.find(haystack)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_subjects_from_topic_and_filename() {
        let tags = infer_subjects(Some("Management of Septic Shock"), "pathophys shock PCOM 2026.pptx");
        assert!(tags.contains(&"septic shock".to_string()));
        assert!(tags.contains(&"shock".to_string()));
    }
}