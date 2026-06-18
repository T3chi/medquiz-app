use regex::Regex;

use crate::models::QuestionOption;

pub const NBME_SYSTEM_RULES: &str = r#"NBME ONE-BEST-ANSWER (A-TYPE) ITEM RULES — MANDATORY COMPLIANCE

FORMAT (Chapter 2):
- Use only one-best-answer A-type items with exactly 5 options (A through E).
- Structure each item as: VIGNETTE (clinical or experimental scenario) + LEAD-IN (single closed question) + OPTION SET.
- The keyed answer is the single BEST option; distractors may be partially correct but less correct than the key.
- Do NOT use true-false, K-type, X-type, or "select all that apply" formats.

RULE 1 — Important testing points (Chapter 5):
- Test important, clinically relevant concepts from the source material, not trivial facts or esoterica ("zebras").
- Focus on common or potentially catastrophic problems learners must recognize.

RULE 2 — Application of knowledge (Chapter 5–6):
- DEFAULT: Assess APPLICATION of knowledge, not isolated fact recall.
- Use clinical vignettes that require integrating findings to reach a diagnosis, next step, mechanism, or management decision.

RULE 3 — Focused, closed lead-in + cover-the-options rule (Chapters 5–6):
- End the vignette with ONE clear, closed lead-in question.
- Use positively phrased lead-ins. Avoid "EXCEPT", "NOT", "least likely", or negative wording.

RULE 4 — Homogeneous, plausible options (Chapters 2, 5):
- ALL five options must address the SAME dimension as the lead-in.
- Keep options CONCISE and PARALLEL in grammatical structure and length.

RULE 5 — Eliminate technical item flaws (Chapter 3):
- No "None of the above" or "All of the above"
- No grammatical cues, clang clues, or length cues on the correct answer

EXPLANATION REQUIREMENTS:
- Explain why the keyed answer is the BEST (most correct) option.
- Briefly explain why each distractor is less correct (not merely "wrong").
"#;

pub fn difficulty_guidance(difficulty: &str) -> &'static str {
    match difficulty {
        "definition" => "COGNITION LEVEL: Foundational recall WITH proper NBME structure.",
        "second_order" => "COGNITION LEVEL: Application — multi-step integration.",
        _ => "COGNITION LEVEL: Application — single-step clinical reasoning.",
    }
}

pub fn validate_question(stem: &str, options: &[QuestionOption], correct_answer: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if options.len() != 5 {
        issues.push(format!("Must have exactly 5 options (found {}).", options.len()));
    }

    let stem_lower = stem.to_lowercase();
    let forbidden = ["except", "not", "least likely", "all of the above", "none of the above"];
    for word in forbidden {
        if stem_lower.contains(word) {
            issues.push(format!("Lead-in contains forbidden pattern: {word}"));
        }
    }

    if !stem.contains('?') {
        issues.push("Stem must include a clear lead-in question ending with '?'.".into());
    }

    let vague = Regex::new(r"(?i)\b(usually|frequently|often|may|could be|is associated with)\b").unwrap();
    if vague.is_match(stem) {
        issues.push("Vignette or lead-in contains vague NBME-prohibited phrasing.".into());
    }

    let forbidden_opts = [
        "all of the above",
        "none of the above",
        "both a and b",
    ];
    for opt in options {
        let lower = opt.text.to_lowercase();
        for phrase in forbidden_opts {
            if lower.contains(phrase) {
                issues.push(format!("Option contains forbidden phrase: '{phrase}'"));
            }
        }
        if opt.text.len() > 120 {
            issues.push("Options must be concise (≤120 chars each).".into());
        }
    }

    if let Some(idx) = options.iter().position(|o| o.label == correct_answer) {
        let correct_len = options[idx].text.len();
        let others: Vec<_> = options
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != idx)
            .map(|(_, o)| o.text.len())
            .collect();
        if let Some(&max_other) = others.iter().max() {
            if correct_len > (max_other as f64 * 1.5) as usize {
                issues.push("Correct option is substantially longer than distractors.".into());
            }
        }
    }

    issues
}