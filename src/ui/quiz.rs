use std::collections::HashMap;

use dioxus::prelude::*;
use keyboard_types::{Code, Key};

use crate::domain::citations::format_citation_display;
use crate::domain::explanations::format_option_explanations;
use crate::models::{difficulty_label, QuizSettings};
use crate::ui::common::{with_db, use_app_state, NavView};

fn nav_back_label(view: NavView) -> &'static str {
    match view {
        NavView::Home => "Back to Home",
        NavView::Materials => "Back to Materials",
        NavView::Bank => "Back to Question Bank",
        NavView::Settings => "Back to Settings",
    }
}

#[component]
pub fn QuizView(
    question_ids: Vec<String>,
    settings: QuizSettings,
    source_file_id: String,
    session_id: String,
    return_to: NavView,
    on_exit: EventHandler<()>,
    on_request_exit: EventHandler<()>,
    on_retry_incorrect: EventHandler<(Vec<String>, String, QuizSettings)>,
) -> Element {
    let state = use_app_state();
    let questions = use_signal(|| {
        with_db(&state, |db| db.get_questions_by_ids(&question_ids)).unwrap_or_default()
    });
    let index = use_signal(|| 0usize);
    let answers = use_signal(HashMap::<String, String>::new);
    let mut selected = use_signal(|| None::<String>);
    let showing_result = use_signal(|| false);
    let complete = use_signal(|| false);

    if complete() {
        let total = questions().len();
        let correct_count = questions()
            .iter()
            .filter(|q| {
                answers()
                    .get(&q.id)
                    .map(|a| a == &q.correct_answer)
                    .unwrap_or(false)
            })
            .count();
        let incorrect_count = total.saturating_sub(correct_count);
        let score = if total > 0 {
            correct_count as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let _ = with_db(&state, |db| db.complete_quiz_session(&session_id, score));
        let last_score = with_db(&state, |db| Ok(db.get_last_quiz_score())).unwrap_or(None);
        let incorrect_ids: Vec<String> = questions()
            .iter()
            .filter(|q| {
                !answers()
                    .get(&q.id)
                    .map(|a| a == &q.correct_answer)
                    .unwrap_or(false)
            })
            .map(|q| q.id.clone())
            .collect();
        let score_class = if score >= 70.0 {
            "text-success"
        } else {
            "text-error"
        };
        let back_label = nav_back_label(return_to);

        return rsx! {
            div { class: "page page--quiz",
                h1 { class: "page-title", "Quiz Complete" }
                div { class: "card mb-16",
                    p { class: "{score_class} score-display", "{score:.0}%  ({correct_count}/{total} correct)" }
                    div { class: "metric-list",
                        if let Some(prev) = last_score {
                            div { class: "metric-row",
                                span { class: "text-secondary", "Previous quiz" }
                                span { class: "metric-value", "{prev:.0}%" }
                            }
                            div { class: "metric-row",
                                span { class: "text-secondary", "Change" }
                                span {
                                    class: if score >= prev { "metric-value text-success" } else { "metric-value text-error" },
                                    if score >= prev { "+" } else { "" }
                                    "{score - prev:.0}%"
                                }
                            }
                        }
                        if incorrect_count > 0 {
                            div { class: "metric-row",
                                span { class: "text-secondary", "Missed" }
                                span { class: "metric-value text-error", "{incorrect_count}" }
                            }
                        }
                    }
                }
                div { class: "results-scroll",
                    for (i, q) in questions().iter().enumerate() {
                        {
                            let ok = answers()
                                .get(&q.id)
                                .map(|a| a == &q.correct_answer)
                                .unwrap_or(false);
                            let body = format_option_explanations(q);
                            let cite = format_citation_display(&q.citation);
                            let full = if cite.is_empty() {
                                body
                            } else {
                                format!("{body}\n\n{cite}")
                            };
                            let border_color = if ok { "var(--success)" } else { "var(--error)" };
                            let status = if ok { "Correct" } else { "Incorrect" };
                            let status_class = if ok { "text-success" } else { "text-error" };
                            rsx! {
                                div {
                                    class: "question-card",
                                    style: "border-color: {border_color}",
                                    p { class: "{status_class}", "Q{i+1} — {status}" }
                                    p { "{q.stem}" }
                                    p {
                                        class: "text-secondary",
                                        "Your answer: {answers().get(&q.id).cloned().unwrap_or_else(|| \"—\".into())} | Correct: {q.correct_answer}"
                                    }
                                    p { class: "text-muted pre-wrap", "{full}" }
                                }
                            }
                        }
                    }
                }
                div { class: "quiz-actions mt-16",
                    if !incorrect_ids.is_empty() {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| {
                                let ids = incorrect_ids.clone();
                                let src = source_file_id.clone();
                                let s = settings.clone();
                                on_retry_incorrect.call((ids, src, s));
                            },
                            "Retry missed ({incorrect_count})"
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| on_exit.call(()),
                        "{back_label}"
                    }
                }
            }
        };
    }

    let q = questions().get(index()).cloned();
    let Some(q) = q else {
        return rsx! {
            div { class: "page page--quiz",
                p { "No questions loaded." }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| on_request_exit.call(()),
                    "Exit"
                }
            }
        };
    };

    let per_q = settings.answer_timing == "per_question";
    let total = questions().len();
    let progress_pct = if total > 0 {
        (index() + 1) as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let tags = if q.subjects.is_empty() {
        q.topic.clone().unwrap_or_default()
    } else {
        q.subjects.join(" · ")
    };

    let mut go_to_index = {
        let mut index = index;
        let mut selected = selected;
        let mut showing_result = showing_result;
        let mut answers = answers;
        let questions = questions;
        move |new_idx: usize| {
            if new_idx >= total {
                return;
            }
            if !per_q {
                if let Some(sel) = selected() {
                    let qid = questions().get(index()).map(|q| q.id.clone());
                    if let Some(qid) = qid {
                        let mut map = answers();
                        map.insert(qid, sel);
                        answers.set(map);
                    }
                }
            }
            index.set(new_idx);
            let Some(q_new) = questions().get(new_idx).cloned() else {
                return;
            };
            if let Some(ans) = answers().get(&q_new.id).cloned() {
                selected.set(Some(ans));
                showing_result.set(per_q);
            } else {
                selected.set(None);
                showing_result.set(false);
            }
        }
    };

    let make_primary_action = || {
        let state = state.clone();
        let q = q.clone();
        let session_id = session_id.clone();
        let mut index = index;
        let mut selected = selected;
        let mut showing_result = showing_result;
        let mut answers = answers;
        let mut complete = complete;
        move || {
            if !showing_result() {
                let Some(sel) = selected() else {
                    return;
                };
                let mut map = answers();
                map.insert(q.id.clone(), sel.clone());
                answers.set(map);
                let _ = with_db(&state, |db| {
                    db.record_question_attempt(&q.id, &sel, sel == q.correct_answer, Some(&session_id))
                });
                if per_q {
                    showing_result.set(true);
                } else if index() + 1 >= total {
                    complete.set(true);
                } else {
                    index.set(index() + 1);
                    selected.set(None);
                }
            } else if index() + 1 >= total {
                complete.set(true);
            } else {
                index.set(index() + 1);
                selected.set(None);
                showing_result.set(false);
            }
        }
    };

    let mut primary_action = make_primary_action();
    let primary_action_key = make_primary_action();

    let option_labels: Vec<String> = q.options.iter().map(|o| o.label.clone()).collect();

    let index_for_keys = index;
    let total_for_keys = total;
    let on_request_exit_key = on_request_exit;
    let handle_keydown = {
        let mut selected = selected;
        let mut primary_action_key = primary_action_key;
        let mut go_to_index = go_to_index;
        let showing_result = showing_result;
        let per_q = per_q;
        let index_for_keys = index_for_keys;
        let total_for_keys = total_for_keys;
        let on_request_exit_key = on_request_exit_key;
        move |e: Event<KeyboardData>| {
            if e.modifiers().ctrl() || e.modifiers().alt() || e.modifiers().meta() {
                return;
            }
            let key = e.key();
            let code = e.code();
            if key == Key::Escape {
                on_request_exit_key.call(());
                e.prevent_default();
                return;
            }
            if key == Key::ArrowLeft || code == Code::ArrowLeft {
                if index_for_keys() > 0 {
                    go_to_index(index_for_keys().saturating_sub(1));
                    e.prevent_default();
                }
                return;
            }
            if key == Key::ArrowRight || code == Code::ArrowRight {
                let can_next = index_for_keys() + 1 < total_for_keys && (!per_q || showing_result());
                if can_next {
                    go_to_index(index_for_keys() + 1);
                    e.prevent_default();
                }
                return;
            }
            let options_locked = showing_result() && per_q;
            if !options_locked {
                let idx_from_key = match key {
                    Key::Character(ref c) => match c.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "5" => Some(4),
                        _ => None,
                    },
                    _ => match code {
                        Code::Digit1 | Code::Numpad1 => Some(0),
                        Code::Digit2 | Code::Numpad2 => Some(1),
                        Code::Digit3 | Code::Numpad3 => Some(2),
                        Code::Digit4 | Code::Numpad4 => Some(3),
                        Code::Digit5 | Code::Numpad5 => Some(4),
                        _ => None,
                    },
                };
                if let Some(idx) = idx_from_key {
                    if let Some(label) = option_labels.get(idx) {
                        selected.set(Some(label.clone()));
                        e.prevent_default();
                        return;
                    }
                }
            }
            if key == Key::Enter {
                primary_action_key();
                e.prevent_default();
            }
        }
    };

    rsx! {
        div {
            class: "page page--quiz",
            tabindex: "0",
            onkeydown: handle_keydown,
            div { class: "quiz-header",
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| on_request_exit.call(()),
                    "← Exit Quiz"
                }
                div {
                    span { class: "badge", "{settings.exam_style}" }
                    span { class: "badge", "{difficulty_label(&settings.difficulty)}" }
                }
            }
            div { class: "progress-wrap",
                div { class: "progress-header",
                    span { "Question {index() + 1} of {total}" }
                    span { class: "text-accent", "{progress_pct as i32}%" }
                }
                div { class: "progress-bar",
                    div { class: "progress-fill", style: "width: {progress_pct}%" }
                }
            }
            div { class: "quiz-nav",
                button {
                    class: "btn btn-secondary",
                    disabled: index() == 0,
                    onclick: move |_| go_to_index(index().saturating_sub(1)),
                    "← Previous"
                }
                span { class: "quiz-nav-center", "Question {index() + 1} of {total}" }
                button {
                    class: "btn btn-secondary",
                    disabled: index() + 1 >= total || (per_q && !showing_result()),
                    onclick: move |_| go_to_index(index() + 1),
                    "Next →"
                }
            }
            div { class: "card quiz-body",
                if !tags.is_empty() {
                    p { class: "text-accent", "{tags}" }
                }
                div { class: "pre-wrap quiz-stem selectable-text", "{q.stem}" }
                for (opt_i, opt) in q.options.iter().enumerate() {
                    {
                        let label = opt.label.clone();
                        let is_sel = selected() == Some(label.clone());
                        let mut class = "option-btn".to_string();
                        if showing_result() {
                            if label == q.correct_answer {
                                class = "option-btn correct".into();
                            } else if Some(&label) == selected().as_ref() && label != q.correct_answer {
                                class = "option-btn incorrect".into();
                            }
                        } else if is_sel {
                            class = "option-btn selected".into();
                        }
                        let key_hint = (opt_i + 1).to_string();
                        rsx! {
                            button {
                                class: "{class}",
                                disabled: showing_result() && per_q,
                                onclick: move |_| selected.set(Some(label.clone())),
                                span { class: "option-key", "{key_hint}" }
                                span { class: "option-text", "{opt.label}.  {opt.text}" }
                            }
                        }
                    }
                }
                if showing_result() && per_q {
                    {
                        let ok = selected() == Some(q.correct_answer.clone());
                        let body = format_option_explanations(&q);
                        let cite = format_citation_display(&q.citation);
                        let full = if cite.is_empty() {
                            body
                        } else {
                            format!("{body}\n\n{cite}")
                        };
                        let expl_class = if ok {
                            "explanation-box explanation-box--success"
                        } else {
                            "explanation-box explanation-box--error"
                        };
                        rsx! {
                            div {
                                class: "{expl_class}",
                                p {
                                    class: "explanation-status",
                                    style: "font-weight:bold",
                                    if ok { "Correct!" } else { "Incorrect — Answer: {q.correct_answer}" }
                                }
                                p { "{full}" }
                            }
                        }
                    }
                }
                p { class: "shortcut-hint",
                    "Tip: "
                    kbd { "1" }
                    "–"
                    kbd { "5" }
                    " select · "
                    kbd { "Enter" }
                    " submit · "
                    kbd { "←" }
                    kbd { "→" }
                    " navigate · "
                    kbd { "Esc" }
                    " exit"
                }
            }
            div { class: "quiz-actions",
                button {
                    class: "btn btn-secondary",
                    disabled: index() == 0,
                    onclick: move |_| go_to_index(index().saturating_sub(1)),
                    "← Previous"
                }
                div { class: "quiz-actions-right",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| primary_action(),
                        if !showing_result() {
                            if !per_q && index() + 1 >= total { "Finish Quiz" }
                            else if per_q { "Submit Answer" }
                            else { "Next Question" }
                        } else if index() + 1 >= total {
                            "View Results"
                        } else {
                            "Next Question"
                        }
                    }
                }
            }
        }
    }
}