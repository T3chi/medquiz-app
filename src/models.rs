use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuestionOption {
    pub label: String,
    pub text: String,
    #[serde(default)]
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Citation {
    #[serde(rename = "filename")]
    pub filename: Option<String>,
    #[serde(rename = "professorName")]
    pub professor_name: Option<String>,
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    #[serde(rename = "sourceFileId")]
    pub source_file_id: String,
    pub stem: String,
    pub options: Vec<QuestionOption>,
    #[serde(rename = "correctAnswer")]
    pub correct_answer: String,
    pub explanation: String,
    pub difficulty: String,
    #[serde(rename = "examStyle")]
    pub exam_style: String,
    pub topic: Option<String>,
    #[serde(default)]
    pub subjects: Vec<String>,
    pub citation: Option<Citation>,
    #[serde(rename = "lastResult", default)]
    pub last_result: Option<String>,
    #[serde(rename = "lastAnsweredAt", default)]
    pub last_answered_at: Option<String>,
    #[serde(rename = "attemptCount", default)]
    pub attempt_count: i32,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub id: String,
    pub filename: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "fileType")]
    pub file_type: String,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: String,
    #[serde(rename = "textLength")]
    pub text_length: usize,
    #[serde(rename = "professorName", default)]
    pub professor_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFileDetail {
    pub id: String,
    pub filename: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "fileType")]
    pub file_type: String,
    #[serde(rename = "textContent")]
    pub text_content: String,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: String,
    #[serde(rename = "professorName", default)]
    pub professor_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct QuizSettings {
    #[serde(rename = "question_count")]
    pub question_count: i32,
    pub difficulty: String,
    #[serde(rename = "answer_timing")]
    pub answer_timing: String,
    #[serde(rename = "exam_style")]
    pub exam_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AppSettings {
    pub api_key: String,
    pub api_base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct UserPreferences {
    pub daily_goal: u32,
    pub default_quiz_settings: QuizSettings,
}

#[derive(Debug, Clone)]
pub struct QuestionAttempt {
    pub id: String,
    pub question_id: String,
    pub selected_answer: String,
    pub is_correct: bool,
    pub answered_at: String,
}

#[derive(Debug, Clone)]
pub struct QuizSession {
    pub id: String,
    pub source_file_id: String,
    pub settings: QuizSettings,
    pub question_ids: Vec<String>,
}

pub const DIFFICULTY_LABELS: &[(&str, &str)] = &[
    ("definition", "Definition"),
    ("first_order", "First Order"),
    ("second_order", "Second Order"),
];

pub const DIFFICULTY_DESCRIPTIONS: &[(&str, &str)] = &[
    (
        "definition",
        "Foundational knowledge with NBME one-best-answer structure",
    ),
    (
        "first_order",
        "Application-level items: clinical vignette with single-step reasoning",
    ),
    (
        "second_order",
        "Multi-step integration, differential diagnosis, or synthesis",
    ),
];

pub fn difficulty_label(key: &str) -> &str {
    DIFFICULTY_LABELS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .unwrap_or(key)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewPool {
    Unanswered,
    Incorrect,
    Mixed,
}

#[derive(Debug, Clone, Default)]
pub struct SubjectStats {
    pub subject: String,
    pub attempted: u32,
    pub correct: u32,
    pub accuracy: f64,
}

#[derive(Debug, Clone, Default)]
pub struct DailyActivity {
    pub date: String,
    pub label: String,
    pub answered: u32,
    pub correct: u32,
    pub app_opens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearnModality {
    MultipleChoice,
    Matching,
    ShortAnswer,
    AnalogyCompletion,
    CreateAnalogy,
    RelationshipArrows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArrowDirection {
    Up,
    Down,
    Associated,
}

impl ArrowDirection {
    pub fn symbol(self) -> &'static str {
        match self {
            ArrowDirection::Up => "↑",
            ArrowDirection::Down => "↓",
            ArrowDirection::Associated => "↔",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ArrowDirection::Up => "Increases",
            ArrowDirection::Down => "Decreases",
            ArrowDirection::Associated => "Associated",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingPair {
    pub left_id: String,
    pub left_text: String,
    pub right_id: String,
    pub right_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipItem {
    pub id: String,
    pub anchor: String,
    pub target: String,
    pub correct_direction: ArrowDirection,
}

#[derive(Debug, Clone)]
pub struct LearnItem {
    pub question_id: String,
    pub modality: LearnModality,
    pub level: u32,
    pub concept_title: String,
    pub prompt: String,
    pub options: Option<Vec<QuestionOption>>,
    pub matching_pairs: Option<Vec<MatchingPair>>,
    pub analogy_choices: Option<Vec<QuestionOption>>,
    pub correct_answer: String,
    pub acceptable_keywords: Vec<String>,
    pub relationships: Option<Vec<RelationshipItem>>,
    pub reference_explanation: String,
}

#[derive(Debug, Clone)]
pub enum LearnResponse {
    SelectedOption(String),
    Matching(HashMap<String, String>),
    Text(String),
    ArrowDirections(HashMap<String, ArrowDirection>),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearnMastery {
    pub question_id: String,
    pub current_level: u32,
    pub consecutive_correct: u32,
    pub highest_level: u32,
    pub updated_at: String,
}

impl LearnMastery {
    pub fn default_for(question_id: &str) -> Self {
        Self {
            question_id: question_id.to_string(),
            current_level: 1,
            consecutive_correct: 0,
            highest_level: 1,
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LearnSettings {
    pub question_count: i32,
    pub source_file_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DashboardAnalytics {
    pub current_streak: u32,
    pub longest_streak: u32,
    pub active_days_total: u32,
    pub total_questions: u32,
    pub unanswered_count: u32,
    pub incorrect_count: u32,
    pub total_attempts: u32,
    pub overall_accuracy: f64,
    pub today_answered: u32,
    pub today_correct: u32,
    pub today_accuracy: f64,
    pub quizzes_completed: u32,
    pub avg_quiz_score: f64,
    pub best_subjects: Vec<SubjectStats>,
    pub worst_subjects: Vec<SubjectStats>,
    pub daily_activity: Vec<DailyActivity>,
}