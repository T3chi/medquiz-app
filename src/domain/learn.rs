use std::collections::HashMap;

use rand::seq::SliceRandom;

use crate::models::{
    ArrowDirection, LearnItem, LearnMastery, LearnModality, LearnResponse, MatchingPair,
    Question, RelationshipItem,
};

pub const CORRECT_TO_ADVANCE: u32 = 2;
pub const LEARN_LEVELS: u32 = 6;

pub fn modality_for_level(level: u32) -> LearnModality {
    match level {
        1 => LearnModality::MultipleChoice,
        2 => LearnModality::Matching,
        3 => LearnModality::ShortAnswer,
        4 => LearnModality::AnalogyCompletion,
        5 => LearnModality::CreateAnalogy,
        _ => LearnModality::RelationshipArrows,
    }
}

pub fn modality_key(modality: &LearnModality) -> &'static str {
    match modality {
        LearnModality::MultipleChoice => "multiple_choice",
        LearnModality::Matching => "matching",
        LearnModality::ShortAnswer => "short_answer",
        LearnModality::AnalogyCompletion => "analogy_completion",
        LearnModality::CreateAnalogy => "create_analogy",
        LearnModality::RelationshipArrows => "relationship_arrows",
    }
}

pub fn modality_label(modality: &LearnModality) -> &'static str {
    match modality {
        LearnModality::MultipleChoice => "Multiple Choice",
        LearnModality::Matching => "Matching",
        LearnModality::ShortAnswer => "Short Answer",
        LearnModality::AnalogyCompletion => "Analogy Completion",
        LearnModality::CreateAnalogy => "Create Analogy",
        LearnModality::RelationshipArrows => "Relationship Arrows",
    }
}

pub fn level_label(level: u32) -> String {
    format!("Level {level} · {}", modality_label(&modality_for_level(level)))
}

pub fn build_learn_item(question: &Question, level: u32, pool: &[Question]) -> LearnItem {
    let modality = modality_for_level(level);
    let concept = concept_title(question);
    let correct_text = correct_option_text(question);
    let reference = question.explanation.clone();

    match modality {
        LearnModality::MultipleChoice => LearnItem {
            question_id: question.id.clone(),
            modality,
            level,
            concept_title: concept.clone(),
            prompt: question.stem.clone(),
            options: Some(question.options.clone()),
            matching_pairs: None,
            analogy_choices: None,
            correct_answer: question.correct_answer.clone(),
            acceptable_keywords: vec![],
            relationships: None,
            reference_explanation: reference,
        },
        LearnModality::Matching => {
            let pairs: Vec<MatchingPair> = question
                .options
                .iter()
                .map(|opt| MatchingPair {
                    left_id: opt.label.clone(),
                    left_text: truncate_chars(&opt.text, 48),
                    right_id: opt.label.clone(),
                    right_text: opt.text.clone(),
                })
                .collect();
            LearnItem {
                question_id: question.id.clone(),
                modality,
                level,
                concept_title: concept.clone(),
                prompt: format!(
                    "Match each label to the correct description for:\n\n{concept}\n\n{stem}",
                    stem = truncate_chars(&question.stem, 200)
                ),
                options: None,
                matching_pairs: Some(pairs),
                analogy_choices: None,
                correct_answer: String::new(),
                acceptable_keywords: vec![],
                relationships: None,
                reference_explanation: reference,
            }
        }
        LearnModality::ShortAnswer => {
            let keywords = extract_keywords(&correct_text, &question.explanation);
            LearnItem {
                question_id: question.id.clone(),
                modality,
                level,
                concept_title: concept.clone(),
                prompt: format!(
                    "Answer in your own words (short phrase or sentence):\n\n{stem}",
                    stem = question.stem
                ),
                options: None,
                matching_pairs: None,
                analogy_choices: None,
                correct_answer: correct_text.clone(),
                acceptable_keywords: keywords,
                relationships: None,
                reference_explanation: reference,
            }
        }
        LearnModality::AnalogyCompletion => {
            let stem_hint = truncate_chars(&question.stem, 80);
            let choices = build_analogy_choices(question, pool);
            LearnItem {
                question_id: question.id.clone(),
                modality,
                level,
                concept_title: concept.clone(),
                prompt: format!(
                    "{concept} is to foundational mechanism as {stem_hint} is to ___",
                ),
                options: None,
                matching_pairs: None,
                analogy_choices: Some(choices.clone()),
                correct_answer: correct_text.clone(),
                acceptable_keywords: extract_keywords(&correct_text, &question.explanation),
                relationships: None,
                reference_explanation: reference,
            }
        }
        LearnModality::CreateAnalogy => {
            let keywords = extract_keywords(&correct_text, &question.explanation);
            LearnItem {
                question_id: question.id.clone(),
                modality,
                level,
                concept_title: concept.clone(),
                prompt: format!(
                    "Create an analogy (2–4 sentences) to explain **{concept}**.\n\n\
                     Your analogy should capture why the best answer is: {correct_text}\n\n\
                     Use a comparison from everyday life that maps the key relationships."
                ),
                options: None,
                matching_pairs: None,
                analogy_choices: None,
                correct_answer: correct_text.clone(),
                acceptable_keywords: keywords,
                relationships: None,
                reference_explanation: reference,
            }
        }
        LearnModality::RelationshipArrows => {
            let relationships = build_relationships(question);
            LearnItem {
                question_id: question.id.clone(),
                modality,
                level,
                concept_title: concept.clone(),
                prompt: format!(
                    "For each pair, choose whether the target **increases (↑)**, **decreases (↓)**, \
                     or is **associated (↔)** with the anchor in the context of {concept}."
                ),
                options: None,
                matching_pairs: None,
                analogy_choices: None,
                correct_answer: String::new(),
                acceptable_keywords: vec![],
                relationships: Some(relationships),
                reference_explanation: reference,
            }
        }
    }
}

pub fn build_learn_session(
    questions: &[Question],
    mastery_map: &HashMap<String, LearnMastery>,
    pool: &[Question],
) -> Vec<LearnItem> {
    let mut items = Vec::new();
    for question in questions {
        let level = mastery_map
            .get(&question.id)
            .map(|m| m.current_level)
            .unwrap_or(1)
            .clamp(1, LEARN_LEVELS);
        items.push(build_learn_item(question, level, pool));
    }
    items
}

pub fn apply_mastery_result(mastery: &mut LearnMastery, is_correct: bool) -> bool {
    mastery.updated_at = chrono::Utc::now().to_rfc3339();
    if is_correct {
        mastery.consecutive_correct += 1;
        if mastery.consecutive_correct >= CORRECT_TO_ADVANCE && mastery.current_level < LEARN_LEVELS {
            mastery.current_level += 1;
            mastery.consecutive_correct = 0;
            mastery.highest_level = mastery.highest_level.max(mastery.current_level);
            return true;
        }
    } else {
        mastery.consecutive_correct = 0;
        if mastery.current_level > 1 {
            mastery.current_level -= 1;
        }
    }
    false
}

pub fn grade_response(item: &LearnItem, response: &LearnResponse) -> bool {
    match (&item.modality, response) {
        (LearnModality::MultipleChoice, LearnResponse::SelectedOption(sel)) => {
            sel == &item.correct_answer
        }
        (LearnModality::Matching, LearnResponse::Matching(map)) => {
            let Some(pairs) = &item.matching_pairs else {
                return false;
            };
            pairs.iter().all(|p| map.get(&p.left_id) == Some(&p.right_id))
        }
        (LearnModality::ShortAnswer, LearnResponse::Text(text)) => {
            grade_text_response(text, &item.correct_answer, &item.acceptable_keywords, 0.45)
        }
        (LearnModality::AnalogyCompletion, LearnResponse::SelectedOption(sel)) => item
            .analogy_choices
            .as_ref()
            .is_some_and(|choices| {
                choices
                    .iter()
                    .any(|c| c.label == *sel && c.text == item.correct_answer)
            }),
        (LearnModality::AnalogyCompletion, LearnResponse::Text(text)) => {
            grade_text_response(text, &item.correct_answer, &item.acceptable_keywords, 0.5)
        }
        (LearnModality::CreateAnalogy, LearnResponse::Text(text)) => {
            text.trim().len() >= 30
                && grade_text_response(text, &item.correct_answer, &item.acceptable_keywords, 0.35)
        }
        (LearnModality::RelationshipArrows, LearnResponse::ArrowDirections(dir_map)) => {
            let Some(rels) = &item.relationships else {
                return false;
            };
            rels.iter().all(|r| dir_map.get(&r.id) == Some(&r.correct_direction))
        }
        _ => false,
    }
}

fn grade_text_response(text: &str, answer: &str, keywords: &[String], threshold: f64) -> bool {
    let normalized = normalize(text);
    if normalized.is_empty() {
        return false;
    }
    let answer_norm = normalize(answer);
    if normalized.contains(&answer_norm) || answer_norm.contains(&normalized) {
        return true;
    }
    if keywords.is_empty() {
        return false;
    }
    let hits = keywords
        .iter()
        .filter(|kw| normalized.contains(&normalize(kw)))
        .count();
    hits as f64 / keywords.len() as f64 >= threshold
}

fn build_analogy_choices(question: &Question, pool: &[Question]) -> Vec<crate::models::QuestionOption> {
    let correct = correct_option_text(question);
    let mut distractors: Vec<String> = question
        .options
        .iter()
        .filter(|o| o.label != question.correct_answer)
        .map(|o| o.text.clone())
        .collect();

    for other in pool {
        if other.id == question.id {
            continue;
        }
        if let Some(text) = other
            .options
            .iter()
            .find(|o| o.label == other.correct_answer)
            .map(|o| o.text.clone())
        {
            if !distractors.contains(&text) && text != correct {
                distractors.push(text);
            }
        }
        if distractors.len() >= 6 {
            break;
        }
    }

    distractors.retain(|d| d != &correct);
    distractors.shuffle(&mut rand::thread_rng());

    let mut choices = vec![crate::models::QuestionOption {
        label: "A".into(),
        text: correct.clone(),
        explanation: String::new(),
    }];
    for (i, text) in distractors.into_iter().take(3).enumerate() {
        choices.push(crate::models::QuestionOption {
            label: char::from(b'B' + i as u8).to_string(),
            text,
            explanation: String::new(),
        });
    }
    choices.shuffle(&mut rand::thread_rng());
    for (i, choice) in choices.iter_mut().enumerate() {
        choice.label = char::from(b'A' + i as u8).to_string();
    }
    choices
}

fn build_relationships(question: &Question) -> Vec<RelationshipItem> {
    let concept = concept_title(question);
    let correct = correct_option_text(question);
    let mut items = vec![
        RelationshipItem {
            id: "r1".into(),
            anchor: concept.clone(),
            target: truncate_chars(&correct, 60),
            correct_direction: infer_direction(&question.explanation, true),
        },
        RelationshipItem {
            id: "r2".into(),
            anchor: truncate_chars(&question.stem, 50),
            target: correct.clone(),
            correct_direction: ArrowDirection::Associated,
        },
    ];

    for (i, opt) in question
        .options
        .iter()
        .filter(|o| o.label != question.correct_answer)
        .take(2)
        .enumerate()
    {
        items.push(RelationshipItem {
            id: format!("r{}", i + 3),
            anchor: concept.clone(),
            target: truncate_chars(&opt.text, 60),
            correct_direction: ArrowDirection::Associated,
        });
    }
    items
}

fn infer_direction(explanation: &str, positive: bool) -> ArrowDirection {
    let lower = explanation.to_lowercase();
    let up_words = ["increase", "elevate", "raise", "stimulate", "enhance", "upregulate"];
    let down_words = ["decrease", "reduce", "lower", "inhibit", "suppress", "downregulate", "block"];

    let up = up_words.iter().any(|w| lower.contains(w));
    let down = down_words.iter().any(|w| lower.contains(w));

    match (up, down, positive) {
        (true, false, _) => ArrowDirection::Up,
        (false, true, _) => ArrowDirection::Down,
        (true, true, true) => ArrowDirection::Up,
        (true, true, false) => ArrowDirection::Down,
        _ => ArrowDirection::Associated,
    }
}

fn concept_title(question: &Question) -> String {
    question
        .subjects
        .first()
        .cloned()
        .or_else(|| question.topic.clone())
        .unwrap_or_else(|| "Core concept".into())
}

fn correct_option_text(question: &Question) -> String {
    question
        .options
        .iter()
        .find(|o| o.label == question.correct_answer)
        .map(|o| o.text.clone())
        .unwrap_or_else(|| question.correct_answer.clone())
}

fn extract_keywords(correct: &str, explanation: &str) -> Vec<String> {
    let mut words: Vec<String> = correct
        .split(|c: char| !c.is_alphanumeric())
        .chain(explanation.split(|c: char| !c.is_alphanumeric()))
        .map(|w| w.trim().to_lowercase())
        .filter(|w| w.len() >= 4)
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .collect();
    words.sort();
    words.dedup();
    words.truncate(12);
    words
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}

const STOP_WORDS: &[&str] = &[
    "that", "this", "with", "from", "which", "have", "been", "were", "their", "there", "about",
    "would", "could", "should", "patient", "following", "most", "likely", "best", "answer",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::QuestionOption;

    fn sample_question() -> Question {
        Question {
            id: "q1".into(),
            source_file_id: "f1".into(),
            stem: "A 45-year-old man has chest pain. Which enzyme is most elevated in MI?".into(),
            options: vec![
                QuestionOption {
                    label: "A".into(),
                    text: "CK-MB".into(),
                    explanation: "Rises after myocardial injury.".into(),
                },
                QuestionOption {
                    label: "B".into(),
                    text: "ALT".into(),
                    explanation: "Hepatic enzyme.".into(),
                },
                QuestionOption {
                    label: "C".into(),
                    text: "Amylase".into(),
                    explanation: "Pancreatic.".into(),
                },
                QuestionOption {
                    label: "D".into(),
                    text: "LDH".into(),
                    explanation: "Less specific.".into(),
                },
                QuestionOption {
                    label: "E".into(),
                    text: "AST".into(),
                    explanation: "Also rises in MI.".into(),
                },
            ],
            correct_answer: "A".into(),
            explanation: "CK-MB increases after myocardial infarction.".into(),
            difficulty: "definition".into(),
            exam_style: "USMLE".into(),
            topic: Some("Cardiology".into()),
            subjects: vec!["Cardiology".into()],
            citation: None,
            last_result: None,
            last_answered_at: None,
            attempt_count: 0,
            created_at: None,
        }
    }

    #[test]
    fn grades_multiple_choice() {
        let item = build_learn_item(&sample_question(), 1, &[]);
        assert!(grade_response(
            &item,
            &LearnResponse::SelectedOption("A".into())
        ));
    }

    #[test]
    fn advances_mastery_after_two_correct() {
        let mut m = LearnMastery::default_for("q1");
        assert!(!apply_mastery_result(&mut m, true));
        assert!(apply_mastery_result(&mut m, true));
        assert_eq!(m.current_level, 2);
    }
}