use dioxus::prelude::*;
use keyboard_types::Key;

use crate::db::Database;
use crate::models::{QuizSettings, ReviewPool};
use crate::AppState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavView {
    Home,
    Materials,
    Bank,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StudyTab {
    Review,
    Practice,
    Learn,
}

#[derive(Clone, PartialEq)]
pub enum Screen {
    Nav(NavView),
    Quiz {
        question_ids: Vec<String>,
        settings: QuizSettings,
        source_file_id: String,
        session_id: String,
        return_to: NavView,
    },
    Learn {
        question_ids: Vec<String>,
        source_file_id: String,
        return_to: NavView,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    DeleteQuestion { id: String },
    DeleteSourceFile { id: String },
    ExitQuiz,
    ExitLearn,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ConfirmRequest {
    pub title: String,
    pub message: String,
    pub action: ConfirmAction,
}

pub fn with_db<T>(state: &AppState, f: impl FnOnce(&Database) -> anyhow::Result<T>) -> anyhow::Result<T> {
    let guard = state.db.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
    f(&guard)
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}

pub fn start_review_quiz(
    state: &AppState,
    pool: ReviewPool,
    count: usize,
    on_start: &EventHandler<(Vec<String>, QuizSettings, String)>,
) {
    let Ok(ids) = with_db(state, |db| db.get_review_question_ids(pool, count)) else {
        return;
    };
    if ids.is_empty() {
        return;
    }
    let Ok(questions) = with_db(state, |db| db.get_questions_by_ids(&ids)) else {
        return;
    };
    let Some(first) = questions.first() else {
        return;
    };
    let settings = QuizSettings {
        question_count: ids.len() as i32,
        ..QuizSettings::default()
    };
    on_start.call((ids, settings, first.source_file_id.clone()));
}

#[component]
pub fn ConfirmDialog(
    request: Option<ConfirmRequest>,
    on_confirm: EventHandler<ConfirmAction>,
    on_close: EventHandler<()>,
) -> Element {
    let Some(req) = request else {
        return rsx! {};
    };
    let action = req.action.clone();
    let action_confirm = action.clone();
    rsx! {
        div {
            class: "modal-overlay",
            tabindex: "0",
            onkeydown: move |e: Event<KeyboardData>| {
                if e.modifiers().ctrl() || e.modifiers().alt() || e.modifiers().meta() {
                    return;
                }
                match e.key() {
                    Key::Escape => {
                        on_close.call(());
                        e.prevent_default();
                    }
                    Key::Enter => {
                        on_confirm.call(action_confirm.clone());
                        on_close.call(());
                        e.prevent_default();
                    }
                    _ => {}
                }
            },
            div { class: "modal-card",
                h3 { "{req.title}" }
                p { class: "text-secondary", "{req.message}" }
                div { class: "modal-actions",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            on_confirm.call(action.clone());
                            on_close.call(());
                        },
                        "Confirm"
                    }
                }
            }
        }
    }
}

#[component]
pub fn Toast(message: Option<String>) -> Element {
    let Some(msg) = message else {
        return rsx! {};
    };
    rsx! {
        div { class: "toast", "{msg}" }
    }
}