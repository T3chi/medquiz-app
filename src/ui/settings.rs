use dioxus::prelude::*;

use crate::config::{self, DEFAULT_DAILY_GOAL};
use crate::models::{AppSettings, UserPreferences, DIFFICULTY_LABELS};
use crate::services::test_api_connection;
use crate::ui::common::{with_db, use_app_state};

#[component]
pub fn SettingsView(on_show_toast: EventHandler<String>) -> Element {
    let state = use_app_state();
    let mut settings = use_signal(AppSettings::default);
    let mut prefs = use_signal(UserPreferences::default);
    let mut status = use_signal(String::new);
    let mut testing = use_signal(|| false);

    let state_for_effect = state.clone();
    use_effect(move || {
        let _ = with_db(&state_for_effect, |db| {
            settings.set(db.get_app_settings()?);
            prefs.set(db.get_user_preferences()?);
            Ok(())
        });
    });

    rsx! {
        div { class: "page page--compact",
            h1 { class: "page-title", "Settings" }
            p { class: "page-subtitle", "AI provider, study defaults, and daily goals." }

            div { class: "card",
                h3 { "AI provider" }
                div { class: "radio-row mb-16",
                    button { class: "btn btn-secondary", onclick: move |_| {
                        settings.with_mut(|s| {
                            s.api_base_url = "https://api.openai.com/v1".into();
                            s.model = "gpt-4o-mini".into();
                        });
                    }, "OpenAI preset" }
                    button { class: "btn btn-secondary", onclick: move |_| {
                        settings.with_mut(|s| {
                            s.api_base_url = "http://localhost:1234/v1".into();
                            s.model = "local-model".into();
                        });
                    }, "LM Studio preset" }
                }
                div { class: "field",
                    label { class: "label", "API key" }
                    input { class: "input", r#type: "password", value: "{settings().api_key}",
                        oninput: move |e| settings.with_mut(|s| s.api_key = e.value()) }
                }
                div { class: "field",
                    label { class: "label", "API base URL" }
                    input { class: "input", value: "{settings().api_base_url}",
                        oninput: move |e| settings.with_mut(|s| s.api_base_url = e.value()) }
                }
                div { class: "field",
                    label { class: "label", "Model" }
                    input { class: "input", value: "{settings().model}",
                        oninput: move |e| settings.with_mut(|s| s.model = e.value()) }
                }
                div { class: "radio-row",
                    button {
                        class: "btn btn-secondary",
                        disabled: testing(),
                        onclick: move |_| {
                            testing.set(true);
                            status.set(String::new());
                            let s = settings();
                            spawn(async move {
                                match test_api_connection(&s).await {
                                    Ok(()) => status.set("Connection successful".into()),
                                    Err(e) => status.set(e.to_string()),
                                }
                                testing.set(false);
                            });
                        },
                        if testing() { "Testing…" } else { "Test connection" }
                    }
                }
                if !status().is_empty() {
                    p {
                        class: if status().contains("successful") { "text-success" } else { "text-error" },
                        "{status()}"
                    }
                }
            }

            div { class: "card mt-16",
                h3 { "Study defaults" }
                div { class: "field",
                    label { class: "label", "Daily question goal" }
                    input {
                        class: "input input-narrow",
                        value: "{prefs().daily_goal}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<u32>() {
                                prefs.with_mut(|p| p.daily_goal = v.clamp(1, 200));
                            }
                        },
                    }
                    p { class: "text-muted", "Shown on Home progress ring (default {DEFAULT_DAILY_GOAL})" }
                }
                div { class: "field",
                    label { class: "label", "Default exam style" }
                    select {
                        class: "select",
                        onchange: move |e| prefs.with_mut(|p| p.default_quiz_settings.exam_style = e.value()),
                        option { value: "USMLE", selected: prefs().default_quiz_settings.exam_style == "USMLE", "USMLE" }
                        option { value: "COMLEX", selected: prefs().default_quiz_settings.exam_style == "COMLEX", "COMLEX" }
                    }
                }
                div { class: "field",
                    label { class: "label", "Default difficulty" }
                    select {
                        class: "select",
                        onchange: move |e| prefs.with_mut(|p| p.default_quiz_settings.difficulty = e.value()),
                        for (k, l) in DIFFICULTY_LABELS {
                            option { value: "{k}", selected: prefs().default_quiz_settings.difficulty == *k, "{l}" }
                        }
                    }
                }
                div { class: "field",
                    label { class: "label", "Default answer timing" }
                    select {
                        class: "select",
                        onchange: move |e| prefs.with_mut(|p| p.default_quiz_settings.answer_timing = e.value()),
                        option { value: "per_question", selected: prefs().default_quiz_settings.answer_timing == "per_question", "After each question" }
                        option { value: "end_of_quiz", selected: prefs().default_quiz_settings.answer_timing == "end_of_quiz", "End of quiz" }
                    }
                }
            }

            button {
                class: "btn btn-primary w-full mt-16",
                onclick: move |_| {
                    let s = settings();
                    let p = prefs();
                    let ok = with_db(&state, |db| {
                        db.save_app_settings(&s)?;
                        db.save_user_preferences(&p)?;
                        Ok(())
                    }).is_ok();
                    if ok {
                        on_show_toast.call("Settings saved".into());
                    }
                },
                "Save settings"
            }

            div { class: "card mt-16",
                h3 { "Keyboard shortcuts" }
                p { class: "text-secondary mb-12",
                    "Full reference: docs/KEYBOARD_SHORTCUTS.md in the project folder."
                }
                table { class: "shortcut-table",
                    thead {
                        tr {
                            th { "Context" }
                            th { "Keys" }
                            th { "Action" }
                        }
                    }
                    tbody {
                        tr {
                            td { "Quiz" }
                            td { "1 – 5" }
                            td { "Select option A – E" }
                        }
                        tr {
                            td { "Quiz" }
                            td { "Enter" }
                            td { "Submit answer or advance" }
                        }
                        tr {
                            td { "Quiz" }
                            td { "← →" }
                            td { "Previous / next question" }
                        }
                        tr {
                            td { "Quiz / Learn" }
                            td { "Esc" }
                            td { "Exit session (confirm)" }
                        }
                        tr {
                            td { "Learn (MCQ)" }
                            td { "1 – 5, Enter" }
                            td { "Select options, check / next" }
                        }
                        tr {
                            td { "Dialogs" }
                            td { "Esc / Enter" }
                            td { "Cancel / confirm" }
                        }
                    }
                }
            }

            div { class: "card mt-16",
                p { class: "text-secondary",
                    "Data stored locally at {config::db_path().display()}"
                }
            }
        }
    }
}