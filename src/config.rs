use std::path::PathBuf;

pub const APP_NAME: &str = "MedQuiz";
pub const MIN_QUESTION_COUNT: i32 = 1;
pub const MAX_QUESTION_COUNT: i32 = 100;

pub fn app_data_dir() -> PathBuf {
    let base = if cfg!(windows) {
        dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()
            .map(|h| h.join("Library/Application Support"))
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        dirs::home_dir()
            .map(|h| h.join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let path = base.join("medquiz-app");
    std::fs::create_dir_all(&path).ok();
    path
}

pub fn db_path() -> PathBuf {
    app_data_dir().join("medquiz.db")
}

pub fn uploads_dir() -> PathBuf {
    let path = app_data_dir().join("uploads");
    std::fs::create_dir_all(&path).ok();
    path
}

pub fn parse_question_count(raw: &str, _fallback: i32) -> Option<i32> {
    let value: i32 = raw.trim().parse().ok()?;
    if (MIN_QUESTION_COUNT..=MAX_QUESTION_COUNT).contains(&value) {
        Some(value)
    } else {
        None
    }
}

pub const LAST_UPLOAD_DIR_KEY: &str = "last_upload_dir";
pub const ONBOARDING_COMPLETE_KEY: &str = "onboarding_complete";
pub const DAILY_GOAL_KEY: &str = "daily_goal";
pub const DEFAULT_QUIZ_SETTINGS_KEY: &str = "default_quiz_settings";
pub const DEFAULT_DAILY_GOAL: u32 = 20;