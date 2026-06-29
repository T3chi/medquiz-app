mod bank;
mod common;
mod home;
mod learn;
mod materials;
mod quiz;
mod settings;
mod sidebar;

pub use common::Screen;

use dioxus::prelude::*;

use crate::db::Database;
use crate::models::QuizSettings;
use crate::ui::common::{ConfirmAction, ConfirmDialog, ConfirmRequest, NavView, Toast};
use crate::AppState;

use self::bank::QuestionBank;
use self::home::Home;
use self::learn::LearnView;
use self::materials::Materials;
use self::quiz::QuizView;
use self::settings::SettingsView;
use self::sidebar::Sidebar;

#[component]
pub fn App() -> Element {
    let state = use_hook(|| {
        let db = Database::open().expect("Failed to open database");
        AppState {
            db: Arc::new(Mutex::new(db)),
        }
    });
    use_context_provider(|| state.clone());

    let mut screen = use_signal(|| Screen::Nav(NavView::Home));
    let bank_refresh = use_signal(|| 0u32);
    let files_refresh = use_signal(|| 0u32);
    let home_refresh = use_signal(|| 0u32);
    let mut confirm = use_signal(|| None::<ConfirmRequest>);
    let toast = use_signal(|| None::<String>);
    let state_for_open = state.clone();
    use_effect(move || {
        let _ = common::with_db(&state_for_open, |db| db.record_app_open());
    });

    let show_toast = {
        let mut toast = toast;
        move |msg: String| {
            toast.set(Some(msg));
        }
    };

    let on_quiz_ready = {
        let state = state.clone();
        let mut screen = screen;
        move |(ids, settings, source_id): (Vec<String>, QuizSettings, String)| {
            if let Ok(session) =
                common::with_db(&state, |db| db.create_quiz_session(&source_id, &settings, &ids))
            {
                screen.set(Screen::Quiz {
                    question_ids: ids,
                    settings,
                    source_file_id: source_id,
                    session_id: session.id,
                    return_to: NavView::Home,
                });
            }
        }
    };

    let on_start_session = {
        let state = state.clone();
        let mut screen = screen;
        move |(ids, settings, source_id, return_to): (Vec<String>, QuizSettings, String, NavView)| {
            if let Ok(session) =
                common::with_db(&state, |db| db.create_quiz_session(&source_id, &settings, &ids))
            {
                screen.set(Screen::Quiz {
                    question_ids: ids,
                    settings,
                    source_file_id: source_id,
                    session_id: session.id,
                    return_to,
                });
            }
        }
    };

    let on_learn_start = {
        let mut screen = screen;
        move |(ids, source_id): (Vec<String>, String)| {
            screen.set(Screen::Learn {
                question_ids: ids,
                source_file_id: source_id,
                return_to: NavView::Home,
            });
        }
    };

    let mut on_session_exit = {
        let mut screen = screen;
        let mut bank_refresh = bank_refresh;
        let mut files_refresh = files_refresh;
        let mut home_refresh = home_refresh;
        move |return_to: NavView| {
            bank_refresh.set(bank_refresh() + 1);
            files_refresh.set(files_refresh() + 1);
            home_refresh.set(home_refresh() + 1);
            screen.set(Screen::Nav(return_to));
        }
    };

    let on_confirm = {
        let state = state.clone();
        let mut screen = screen;
        let mut bank_refresh = bank_refresh;
        let mut files_refresh = files_refresh;
        let mut home_refresh = home_refresh;
        move |action: ConfirmAction| {
            match action {
                ConfirmAction::DeleteQuestion { id } => {
                    let _ = common::with_db(&state, |db| db.delete_question(&id));
                    bank_refresh.set(bank_refresh() + 1);
                    home_refresh.set(home_refresh() + 1);
                }
                ConfirmAction::DeleteSourceFile { id } => {
                    let _ = common::with_db(&state, |db| db.delete_source_file(&id));
                    files_refresh.set(files_refresh() + 1);
                }
                ConfirmAction::ExitQuiz => {
                    if let Screen::Quiz { return_to, .. } = screen() {
                        bank_refresh.set(bank_refresh() + 1);
                        home_refresh.set(home_refresh() + 1);
                        screen.set(Screen::Nav(return_to));
                    }
                }
                ConfirmAction::ExitLearn => {
                    if let Screen::Learn { return_to, .. } = screen() {
                        home_refresh.set(home_refresh() + 1);
                        screen.set(Screen::Nav(return_to));
                    }
                }
            }
        }
    };

    let mut request_confirm = {
        let mut confirm = confirm;
        move |req: ConfirmRequest| confirm.set(Some(req))
    };

    rsx! {
        document::Style { {include_str!("../../assets/style.css")} }
        ConfirmDialog {
            request: confirm(),
            on_confirm: on_confirm,
            on_close: move |_| confirm.set(None),
        }
        Toast { message: toast() }
        div { class: "app-shell",
            Sidebar {
                current: match screen() {
                    Screen::Nav(v) => v,
                    Screen::Quiz { .. } => NavView::Home,
                    Screen::Learn { .. } => NavView::Home,
                },
                session_active: matches!(screen(), Screen::Quiz { .. } | Screen::Learn { .. }),
                api_ready: common::with_db(&state, |db| Ok(db.api_configured())).unwrap_or(false),
                on_nav: move |v| screen.set(Screen::Nav(v)),
            }
            div { class: "content",
                match screen() {
                    Screen::Nav(NavView::Home) => rsx! {
                        Home {
                            key: "{home_refresh()}",
                            on_start: on_start_session,
                            on_learn_start,
                            on_nav_materials: move |_| screen.set(Screen::Nav(NavView::Materials)),
                            on_show_toast: show_toast,
                        }
                    },
                    Screen::Nav(NavView::Materials) => rsx! {
                        Materials {
                            key: "{files_refresh()}",
                            on_quiz_ready,
                            on_nav_settings: move |_| screen.set(Screen::Nav(NavView::Settings)),
                            request_confirm,
                        }
                    },
                    Screen::Nav(NavView::Bank) => rsx! {
                        QuestionBank {
                            key: "{bank_refresh()}",
                            on_start: on_start_session,
                            request_confirm,
                        }
                    },
                    Screen::Nav(NavView::Settings) => rsx! {
                        SettingsView {
                            on_show_toast: show_toast,
                        }
                    },
                    Screen::Quiz { question_ids, settings, source_file_id, session_id, return_to } => rsx! {
                        QuizView {
                            question_ids,
                            settings,
                            source_file_id,
                            session_id,
                            return_to,
                            on_exit: move |_| on_session_exit(return_to),
                            on_request_exit: move |_| {
                                request_confirm(ConfirmRequest {
                                    title: "Leave quiz?".into(),
                                    message: "Your answers in this session will be kept, but you'll return home.".into(),
                                    action: ConfirmAction::ExitQuiz,
                                });
                            },
                            on_retry_incorrect: {
                                let state = state.clone();
                                let mut screen = screen;
                                move |(ids, src, s): (Vec<String>, String, QuizSettings)| {
                                    if let Ok(session) = common::with_db(&state, |db| {
                                        db.create_quiz_session(&src, &s, &ids)
                                    }) {
                                        screen.set(Screen::Quiz {
                                            question_ids: ids,
                                            settings: s,
                                            source_file_id: src,
                                            session_id: session.id,
                                            return_to,
                                        });
                                    }
                                }
                            },
                        }
                    },
                    Screen::Learn { question_ids, source_file_id, return_to } => rsx! {
                        LearnView {
                            question_ids,
                            source_file_id,
                            return_to,
                            on_exit: move |_| on_session_exit(return_to),
                            on_request_exit: move |_| {
                                request_confirm(ConfirmRequest {
                                    title: "Leave learn session?".into(),
                                    message: "Your mastery progress is saved.".into(),
                                    action: ConfirmAction::ExitLearn,
                                });
                            },
                        }
                    },
                }
            }
        }
    }
}

use std::sync::{Arc, Mutex};