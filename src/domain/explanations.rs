use crate::models::Question;

pub fn options_have_explanations(options: &[crate::models::QuestionOption]) -> bool {
    options.len() == 5 && options.iter().all(|o| !o.explanation.trim().is_empty())
}

pub fn format_option_explanations(question: &Question) -> String {
    let mut sections = Vec::new();
    if !question.explanation.trim().is_empty() {
        sections.push(question.explanation.trim().to_string());
    }
    for opt in &question.options {
        let header = if opt.label == question.correct_answer {
            format!("{}. {} (Correct)", opt.label, opt.text)
        } else {
            format!("{}. {}", opt.label, opt.text)
        };
        let detail = if opt.explanation.trim().is_empty() {
            if opt.label == question.correct_answer {
                "This is the best answer.".to_string()
            } else {
                "This option is incorrect.".to_string()
            }
        } else {
            opt.explanation.trim().to_string()
        };
        sections.push(format!("{header}\n{detail}"));
    }
    sections.join("\n\n")
}