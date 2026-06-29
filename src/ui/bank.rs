use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use crate::domain::learn::LEARN_LEVELS;
use crate::models::{difficulty_label, Question, QuizSettings};
use crate::ui::common::{ConfirmAction, ConfirmRequest, NavView, with_db, use_app_state};

#[derive(Clone, Copy, PartialEq, Eq)]
enum BankFilter {
    All,
    Unanswered,
    Incorrect,
    Mastered,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BankSort {
    Recent,
    MostMissed,
    NeverSeen,
}

#[component]
pub fn QuestionBank(
    on_start: EventHandler<(Vec<String>, QuizSettings, String, NavView)>,
    request_confirm: EventHandler<ConfirmRequest>,
) -> Element {
    let state = use_app_state();
    let mut all_questions = use_signal(Vec::<Question>::new);
    let mut mastery = use_signal(HashMap::<String, u32>::new);
    let mut search = use_signal(String::new);
    let mut expanded = use_signal(|| None::<String>);
    let mut filter = use_signal(|| BankFilter::All);
    let mut sort = use_signal(|| BankSort::Recent);
    let mut selected_ids = use_signal(HashSet::<String>::new);

    let state_for_effect = state.clone();
    use_effect(move || {
        let _ = with_db(&state_for_effect, |db| {
            all_questions.set(db.get_questions(None, None, None)?);
            let map = db.get_learn_mastery_map()?;
            mastery.set(map.into_iter().map(|(k, v)| (k, v.current_level)).collect());
            Ok(())
        });
    });

    let query = search().trim().to_lowercase();
    let mut questions: Vec<Question> = all_questions()
        .into_iter()
        .filter(|q| {
            let level = mastery().get(&q.id).copied().unwrap_or(1);
            match filter() {
                BankFilter::All => true,
                BankFilter::Unanswered => q.attempt_count == 0,
                BankFilter::Incorrect => q.last_result.as_deref() == Some("incorrect"),
                BankFilter::Mastered => level >= LEARN_LEVELS,
            }
        })
        .filter(|q| {
            if query.is_empty() {
                true
            } else {
                q.subjects.iter().any(|t| t.to_lowercase().contains(&query))
                    || q.stem.to_lowercase().contains(&query)
                    || q.topic.as_ref().map(|t| t.to_lowercase().contains(&query)).unwrap_or(false)
            }
        })
        .collect();

    match sort() {
        BankSort::Recent => questions.sort_by(|a, b| b.last_answered_at.cmp(&a.last_answered_at)),
        BankSort::MostMissed => questions.sort_by(|a, b| {
            let am = a.last_result.as_deref() == Some("incorrect");
            let bm = b.last_result.as_deref() == Some("incorrect");
            bm.cmp(&am)
        }),
        BankSort::NeverSeen => questions.sort_by(|a, b| a.attempt_count.cmp(&b.attempt_count)),
    }

    rsx! {
        div { class: "page page--wide",
            h1 { class: "page-title", "Question Bank" }
            p { class: "page-subtitle", "{all_questions().len()} questions · {selected_ids().len()} selected" }

            div { class: "chip-row mb-12",
                for (f, label) in [
                    (BankFilter::All, "All"),
                    (BankFilter::Unanswered, "Unanswered"),
                    (BankFilter::Incorrect, "Incorrect"),
                    (BankFilter::Mastered, "Mastered"),
                ] {
                    button {
                        class: if filter() == f { "tag active" } else { "tag" },
                        onclick: move |_| filter.set(f),
                        "{label}"
                    }
                }
            }
            div { class: "search-row mb-12",
                input {
                    class: "input",
                    placeholder: "Search stem or subject…",
                    value: "{search()}",
                    oninput: move |e| search.set(e.value()),
                }
                select {
                    class: "select",
                    onchange: move |e| {
                        sort.set(match e.value().as_str() {
                            "missed" => BankSort::MostMissed,
                            "new" => BankSort::NeverSeen,
                            _ => BankSort::Recent,
                        });
                    },
                    option { value: "recent", "Sort: Recent" }
                    option { value: "missed", "Sort: Most missed" }
                    option { value: "new", "Sort: Never seen" }
                }
            }
            if !selected_ids().is_empty() {
                div { class: "bulk-bar mb-12",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let ids: Vec<String> = selected_ids().into_iter().collect();
                            if ids.is_empty() { return; }
                            let Ok(qs) = with_db(&state, |db| db.get_questions_by_ids(&ids)) else { return };
                            let Some(f) = qs.first() else { return };
                            let s = QuizSettings { question_count: ids.len() as i32, ..QuizSettings::default() };
                            on_start.call((ids, s, f.source_file_id.clone(), NavView::Bank));
                        },
                        "Quiz selected ({selected_ids().len()})"
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| selected_ids.set(HashSet::new()),
                        "Clear selection"
                    }
                }
            }
            if questions.is_empty() {
                div { class: "empty-state", "No questions match your filters." }
            }
            div { class: "question-list",
                for (i, q) in questions.iter().enumerate() {
                    {
                        let qid = q.id.clone();
                        let qid_select = qid.clone();
                        let qid_delete = qid.clone();
                        let qid_expand = qid.clone();
                        let is_sel = selected_ids().contains(&qid);
                        let level = mastery().get(&qid).copied().unwrap_or(1);
                        let stem_preview: String = q.stem.chars().take(200).collect();
                        let stem_preview = if q.stem.len() > 200 { format!("{stem_preview}...") } else { stem_preview };
                        let is_exp = expanded() == Some(qid.clone());
                        let (status_pill, status_class) = if q.attempt_count == 0 {
                            ("New", "status-pill status-pill--new")
                        } else if q.last_result.as_deref() == Some("incorrect") {
                            ("Missed", "status-pill status-pill--missed")
                        } else if level >= LEARN_LEVELS {
                            ("Mastered", "status-pill status-pill--mastered")
                        } else {
                            ("Practiced", "status-pill status-pill--practiced")
                        };
                        rsx! {
                            div {
                                class: if is_exp { "question-card expanded" } else { "question-card" },
                                key: "{q.id}",
                                div { class: "flex-between",
                                    input {
                                        r#type: "checkbox",
                                        checked: is_sel,
                                        onchange: move |_| {
                                            selected_ids.with_mut(|s| {
                                                if s.contains(&qid_select) { s.remove(&qid_select); } else { s.insert(qid_select.clone()); }
                                            });
                                        },
                                    }
                                    span { class: "{status_class}", "{status_pill}" }
                                    span { class: "badge", "Learn L{level}/{LEARN_LEVELS}" }
                                    span { class: "text-secondary", "{q.exam_style} · {difficulty_label(&q.difficulty)}" }
                                    button {
                                        class: "btn btn-ghost ml-auto",
                                        onclick: move |_| {
                                            request_confirm.call(ConfirmRequest {
                                                title: "Delete question?".into(),
                                                message: "This cannot be undone.".into(),
                                                action: ConfirmAction::DeleteQuestion { id: qid_delete.clone() },
                                            });
                                        },
                                        "Delete"
                                    }
                                }
                                button {
                                    class: "stem-preview",
                                    onclick: move |_| {
                                        if expanded() == Some(qid_expand.clone()) { expanded.set(None); }
                                        else { expanded.set(Some(qid_expand.clone())); }
                                    },
                                    "#{questions.len() - i} — {stem_preview}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}