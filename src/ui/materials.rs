use dioxus::prelude::*;

use crate::config::{self, parse_question_count, LAST_UPLOAD_DIR_KEY};
use crate::db::copy_upload;
use crate::models::{QuizSettings, SourceFile, DIFFICULTY_DESCRIPTIONS, DIFFICULTY_LABELS};
use crate::parsers::parse_file;
use crate::services::QuizService;
use crate::ui::common::{ConfirmAction, ConfirmRequest, with_db, use_app_state};

#[component]
pub fn Materials(
    on_quiz_ready: EventHandler<(Vec<String>, QuizSettings, String)>,
    on_nav_settings: EventHandler<()>,
    request_confirm: EventHandler<ConfirmRequest>,
) -> Element {
    let state = use_app_state();
    let mut step = use_signal(|| 0u8);
    let mut files = use_signal(Vec::<SourceFile>::new);
    let mut selected = use_signal(|| None::<String>);
    let mut settings = use_signal(QuizSettings::default);
    let mut count_input = use_signal(|| "10".to_string());
    let mut generating = use_signal(|| false);
    let mut progress_msg = use_signal(String::new);
    let mut progress_pct = use_signal(|| 0.0);
    let mut error_msg = use_signal(String::new);
    let mut generated_ids = use_signal(|| None::<(Vec<String>, QuizSettings, String)>);

    let state_for_effect = state.clone();
    use_effect(move || {
        let _ = with_db(&state_for_effect, |db| {
            files.set(db.get_source_files()?);
            settings.set(db.get_user_preferences()?.default_quiz_settings);
            if selected().is_none() {
                if let Some(f) = files().first() {
                    selected.set(Some(f.id.clone()));
                }
            }
            Ok(())
        });
    });

    let api_ready = with_db(&state, |db| Ok(db.api_configured())).unwrap_or(false);

    if let Some((ids, s, src)) = generated_ids() {
        return rsx! {
            div { class: "page page--wide",
                div { class: "card cta-card",
                    h1 { class: "page-title", "Questions ready" }
                    p { class: "text-success", "Generated {ids.len()} questions and added them to your bank." }
                    div { class: "quick-actions",
                        button {
                            class: "btn btn-primary w-full",
                            onclick: move |_| on_quiz_ready.call((ids.clone(), s.clone(), src.clone())),
                            "Start quiz now"
                        }
                        button {
                            class: "btn btn-secondary w-full",
                            onclick: move |_| generated_ids.set(None),
                            "Generate more"
                        }
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "page page--wide",
            h1 { class: "page-title", "Materials" }
            p { class: "page-subtitle", "Upload lectures and generate board-style questions for your bank." }

            div { class: "wizard-steps mb-16",
                for (i, label) in ["1. Upload", "2. Basics", "3. Advanced"].iter().enumerate() {
                    span {
                        class: if step() as usize == i { "wizard-step active" } else { "wizard-step" },
                        "{label}"
                    }
                }
            }

            if !error_msg().is_empty() {
                div { class: "error-banner",
                    p { class: "text-error", "{error_msg()}" }
                    if !api_ready {
                        button { class: "btn btn-secondary btn-compact", onclick: move |_| on_nav_settings.call(()), "Open Settings" }
                    }
                }
            }

            match step() {
                0 => rsx! {
                    div { class: "card",
                        h3 { "Study materials" }
                        button {
                            class: "btn btn-secondary w-full",
                            style: "margin: 12px 0;",
                            disabled: generating(),
                            onclick: {
                                let state = state.clone();
                                move |_| upload_file(state.clone(), files, selected, error_msg)
                            },
                            "Upload PDF or PowerPoint"
                        }
                        div { class: "scroll-list",
                            for file in files() {
                                {
                                    let id = file.id.clone();
                                    let delete_id = id.clone();
                                    let is_sel = selected().as_deref() == Some(&id);
                                    rsx! {
                                        div {
                                            key: "{file.id}",
                                            class: if is_sel { "file-row selected" } else { "file-row" },
                                            onclick: move |_| selected.set(Some(id.clone())),
                                            span { "[{file.file_type.to_uppercase()}] {file.filename}" }
                                            span { class: "text-muted", "{file.text_length / 1000}k chars" }
                                            button {
                                                class: "btn btn-ghost ml-auto",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    request_confirm.call(ConfirmRequest {
                                                        title: "Delete file?".into(),
                                                        message: "This removes the file and all questions generated from it.".into(),
                                                        action: ConfirmAction::DeleteSourceFile { id: delete_id.clone() },
                                                    });
                                                },
                                                "Delete"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if files().is_empty() {
                            p { class: "text-muted mt-12", "No files yet. Upload a lecture to begin." }
                        }
                    }
                },
                1 => rsx! {
                    div { class: "card",
                        h3 { "Quiz basics" }
                        div { class: "field",
                            label { class: "label", "Number of questions" }
                            input {
                                class: "input input-narrow",
                                value: "{count_input()}",
                                oninput: move |e| count_input.set(e.value()),
                                onblur: move |_| {
                                    if parse_question_count(&count_input(), 10).is_none() {
                                        error_msg.set(format!("Enter {}–{}", config::MIN_QUESTION_COUNT, config::MAX_QUESTION_COUNT));
                                    } else {
                                        error_msg.set(String::new());
                                    }
                                },
                            }
                        }
                        div { class: "field",
                            label { class: "label", "Exam style" }
                            div { class: "radio-row",
                                for style in ["USMLE", "COMLEX"] {
                                    label { class: "radio-label",
                                        input {
                                            r#type: "radio",
                                            checked: settings().exam_style == style,
                                            onchange: move |_| settings.with_mut(|s| s.exam_style = style.to_string()),
                                        }
                                        "{style}"
                                    }
                                }
                            }
                        }
                    }
                },
                _ => rsx! {
                    div { class: "card",
                        h3 { "Advanced options" }
                        div { class: "field",
                            label { class: "label", "Difficulty" }
                            for (key, label) in DIFFICULTY_LABELS {
                                {
                                    let key = key.to_string();
                                    let desc = DIFFICULTY_DESCRIPTIONS.iter().find(|(k,_)| *k == key).map(|(_,d)| *d).unwrap_or("");
                                    rsx! {
                                        label { class: "radio-label", style: "display:block; margin-bottom:8px;",
                                            input {
                                                r#type: "radio",
                                                checked: settings().difficulty == key,
                                                onchange: move |_| settings.with_mut(|s| s.difficulty = key.clone()),
                                            }
                                            "{label} — {desc}"
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "field",
                            label { class: "label", "Show answers" }
                            div { class: "radio-row",
                                label { class: "radio-label",
                                    input { r#type: "radio", checked: settings().answer_timing == "per_question",
                                        onchange: move |_| settings.with_mut(|s| s.answer_timing = "per_question".into()) }
                                    "After each question"
                                }
                                label { class: "radio-label",
                                    input { r#type: "radio", checked: settings().answer_timing == "end_of_quiz",
                                        onchange: move |_| settings.with_mut(|s| s.answer_timing = "end_of_quiz".into()) }
                                    "End of quiz"
                                }
                            }
                        }
                    }
                },
            }

            if generating() {
                div { class: "progress-wrap",
                    div { class: "progress-header",
                        span { "{progress_msg()}" }
                        span { class: "text-accent", "{progress_pct() as i32}%" }
                    }
                    div { class: "progress-bar",
                        div { class: "progress-fill", style: "width: {progress_pct()}%" }
                    }
                }
            }

            div { class: "wizard-nav",
                if step() > 0 {
                    button { class: "btn btn-secondary", disabled: generating(), onclick: move |_| step.set(step() - 1), "Back" }
                }
                if step() < 2 {
                    button {
                        class: "btn btn-primary",
                        disabled: step() == 0 && selected().is_none(),
                        onclick: move |_| step.set(step() + 1),
                        "Next"
                    }
                } else {
                    button {
                        class: "btn btn-primary",
                        disabled: generating() || selected().is_none() || !api_ready,
                        onclick: move |_| {
                            if !api_ready {
                                error_msg.set("Configure your API key in Settings before generating.".into());
                                return;
                            }
                            let count = parse_question_count(&count_input(), settings().question_count);
                            let Some(count) = count else {
                                error_msg.set(format!("Enter a number between {} and {}", config::MIN_QUESTION_COUNT, config::MAX_QUESTION_COUNT));
                                return;
                            };
                            error_msg.set(String::new());
                            let mut s = settings();
                            s.question_count = count;
                            settings.set(s);
                            let Some(source_id) = selected() else { return };
                            let state = state.clone();
                            let s = settings();
                            generating.set(true);
                            spawn(async move {
                                let app_settings = with_db(&state, |db| db.get_app_settings()).unwrap_or_default();
                                let service = QuizService::new(state.db.clone());
                                let result = service.generate_quiz(&source_id, &s, &app_settings, |msg, pct| {
                                    progress_msg.set(msg.to_string());
                                    progress_pct.set(pct);
                                }).await;
                                generating.set(false);
                                match result {
                                    Ok(ids) => generated_ids.set(Some((ids, s, source_id))),
                                    Err(e) => error_msg.set(e.to_string()),
                                }
                            });
                        },
                        if generating() { "Generating…" } else { "Generate questions" }
                    }
                }
            }
        }
    }
}

fn upload_file(
    state: crate::AppState,
    mut files: Signal<Vec<SourceFile>>,
    mut selected: Signal<Option<String>>,
    mut error_msg: Signal<String>,
) {
    spawn(async move {
        let last_dir = with_db(&state, |db| db.get_preference(LAST_UPLOAD_DIR_KEY)).ok();
        let mut dialog = rfd::FileDialog::new().add_filter("Study Materials", &["pdf", "pptx", "ppt"]);
        if let Some(dir) = last_dir.filter(|d| std::path::Path::new(d).is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.pick_file() else { return };
        if let Some(parent) = path.parent() {
            let _ = with_db(&state, |db| db.set_preference(LAST_UPLOAD_DIR_KEY, &parent.to_string_lossy()));
        }
        match parse_file(&path) {
            Ok((text, file_type)) => {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                match copy_upload(&path, &config::uploads_dir()) {
                    Ok(dest) => {
                        if let Ok(record) = with_db(&state, |db| {
                            db.save_source_file(&filename, &dest.to_string_lossy(), &file_type, &text)
                        }) {
                            selected.set(Some(record.id.clone()));
                            if let Ok(list) = with_db(&state, |db| db.get_source_files()) {
                                files.set(list);
                            }
                        }
                    }
                    Err(e) => error_msg.set(e.to_string()),
                }
            }
            Err(e) => error_msg.set(e.to_string()),
        }
    });
}