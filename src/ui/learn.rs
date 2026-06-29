use std::collections::HashMap;

use dioxus::prelude::*;
use keyboard_types::{Code, Key};
use rand::seq::SliceRandom;

use crate::domain::learn::{
    build_learn_session, grade_response, modality_key, modality_label, LEARN_LEVELS,
};
use crate::models::{
    ArrowDirection, LearnItem, LearnModality, LearnResponse,
};
use crate::ui::common::{with_db, use_app_state, NavView};

fn nav_back_label(view: NavView) -> &'static str {
    match view {
        NavView::Home => "Back to Home",
        NavView::Materials => "Back to Materials",
        NavView::Bank => "Back to Question Bank",
        NavView::Settings => "Back to Settings",
    }
}

fn learn_encouragement(correct: u32, total: usize, level_ups: u32) -> &'static str {
    let pct = if total > 0 {
        correct as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    if level_ups > 0 && pct >= 80.0 {
        "Outstanding! You leveled up and showed real mastery today."
    } else if level_ups > 0 {
        "Great progress — harder modalities are now unlocked for several concepts."
    } else if pct >= 90.0 {
        "Excellent session! Your recall is getting sharper."
    } else if pct >= 70.0 {
        "Solid work — keep building on what you got right."
    } else if pct >= 50.0 {
        "Good effort! Review the reference notes and come back stronger."
    } else {
        "Every rep strengthens memory — consistency beats perfection."
    }
}

#[component]
pub fn LearnView(
    question_ids: Vec<String>,
    source_file_id: String,
    return_to: NavView,
    on_exit: EventHandler<()>,
    on_request_exit: EventHandler<()>,
) -> Element {
    let _source_file_id = source_file_id;
    let state = use_app_state();
    let mut items = use_signal(Vec::<LearnItem>::new);
    let index = use_signal(|| 0usize);
    let showing_result = use_signal(|| false);
    let last_correct = use_signal(|| false);
    let leveled_up = use_signal(|| false);
    let complete = use_signal(|| false);
    let correct_count = use_signal(|| 0u32);
    let level_ups = use_signal(|| 0u32);

    let mut selected = use_signal(|| None::<String>);
    let mut text_answer = use_signal(String::new);
    let mut matching = use_signal(HashMap::<String, String>::new);
    let mut arrows = use_signal(HashMap::<String, ArrowDirection>::new);
    let mut shuffled_rights = use_signal(Vec::<(String, String)>::new);

    let ids_for_effect = question_ids.clone();
    let state_for_effect = state.clone();
    use_effect(move || {
        let _ = with_db(&state_for_effect, |db| {
            let questions = db.get_questions_by_ids(&ids_for_effect)?;
            let pool = db.get_questions(None, None, None)?;
            let mastery = db.get_learn_mastery_map()?;
            items.set(build_learn_session(&questions, &mastery, &pool));
            Ok(())
        });
    });

    if complete() {
        let total = items().len();
        let encouragement = learn_encouragement(correct_count(), total, level_ups());
        let back_label = nav_back_label(return_to);
        return rsx! {
            div { class: "page page--quiz",
                h1 { class: "page-title", "Learn Session Complete" }
                div { class: "card",
                    p { class: "text-accent mb-12", "{encouragement}" }
                    div { class: "metric-list",
                        div { class: "metric-row",
                            span { class: "text-secondary", "Items completed" }
                            span { class: "metric-value", "{total}" }
                        }
                        div { class: "metric-row",
                            span { class: "text-secondary", "Correct" }
                            span { class: "metric-value text-success", "{correct_count()}" }
                        }
                        div { class: "metric-row",
                            span { class: "text-secondary", "Level-ups" }
                            span { class: "metric-value text-accent", "{level_ups()}" }
                        }
                        if total > 0 {
                            div { class: "metric-row",
                                span { class: "text-secondary", "Accuracy" }
                                span {
                                    class: "metric-value",
                                    "{correct_count() as f64 / total as f64 * 100.0:.0}%"
                                }
                            }
                        }
                    }
                    p { class: "text-muted mt-16",
                        "Keep practicing — each concept unlocks harder modalities as you demonstrate understanding."
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

    let total = items().len();
    if total == 0 {
        return rsx! {
            div { class: "page page--quiz",
                p { "Loading learn session..." }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| on_request_exit.call(()),
                    "Cancel"
                }
            }
        };
    }

    let current_idx = index().min(total.saturating_sub(1));
    let item = items().get(current_idx).cloned();
    let Some(item) = item else {
        return rsx! {
            div { class: "page page--quiz",
                p { "Session error." }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| on_request_exit.call(()),
                    "Exit"
                }
            }
        };
    };

    if shuffled_rights().is_empty() {
        if let Some(pairs) = &item.matching_pairs {
            let mut rights: Vec<(String, String)> = pairs
                .iter()
                .map(|p| (p.right_id.clone(), p.right_text.clone()))
                .collect();
            rights.shuffle(&mut rand::thread_rng());
            shuffled_rights.set(rights);
        }
    }

    let progress_pct = if total > 0 {
        (current_idx + 1) as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    let option_labels: Vec<String> = match item.modality {
        LearnModality::MultipleChoice => item
            .options
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|o| o.label)
            .collect(),
        LearnModality::AnalogyCompletion => item
            .analogy_choices
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|o| o.label)
            .collect(),
        _ => Vec::new(),
    };

    let make_advance = || {
        let state = state.clone();
        let item = item.clone();
        let mut index = index;
        let mut showing_result = showing_result;
        let mut last_correct = last_correct;
        let mut leveled_up = leveled_up;
        let mut complete = complete;
        let mut correct_count = correct_count;
        let mut level_ups = level_ups;
        let mut selected = selected;
        let mut text_answer = text_answer;
        let mut matching = matching;
        let mut arrows = arrows;
        let mut shuffled_rights = shuffled_rights;
        let mut items = items;
        move || {
            if !showing_result() {
                let response = build_learn_response(
                    &item,
                    selected(),
                    text_answer(),
                    matching(),
                    arrows(),
                );
                let Some(response) = response else {
                    return;
                };
                let ok = grade_response(&item, &response);
                let response_str = learn_response_summary(&response);
                let mut leveled = false;
                if let Ok((_, up)) = with_db(&state, |db| {
                    db.record_learn_attempt(
                        &item.question_id,
                        modality_key(&item.modality),
                        item.level,
                        &response_str,
                        ok,
                    )
                }) {
                    leveled = up;
                }
                last_correct.set(ok);
                leveled_up.set(leveled);
                if ok {
                    correct_count.set(correct_count() + 1);
                }
                if leveled {
                    level_ups.set(level_ups() + 1);
                    if item.level < LEARN_LEVELS {
                        if let Ok(Some(followup)) = with_db(&state, |db| {
                            db.build_learn_followup_item(&item.question_id, item.level + 1)
                        }) {
                            items.with_mut(|list| {
                                let insert_at = (index() + 1).min(list.len());
                                list.insert(insert_at, followup);
                            });
                        }
                    }
                }
                showing_result.set(true);
            } else if index() + 1 >= items().len() {
                complete.set(true);
            } else {
                index.set(index() + 1);
                showing_result.set(false);
                leveled_up.set(false);
                selected.set(None);
                text_answer.set(String::new());
                matching.set(HashMap::new());
                arrows.set(HashMap::new());
                shuffled_rights.set(Vec::new());
            }
        }
    };

    let mut advance = make_advance();
    let advance_key = make_advance();
    let on_request_exit_key = on_request_exit;
    let handle_keydown = {
        let mut selected = selected;
        let mut advance_key = advance_key;
        let showing_result = showing_result;
        let modality = item.modality;
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
            let options_active = matches!(
                modality,
                LearnModality::MultipleChoice | LearnModality::AnalogyCompletion
            );
            if options_active && !showing_result() {
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
                advance_key();
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
                    "← Exit Learn"
                }
                div {
                    span { class: "badge learn-badge", "{modality_label(&item.modality)}" }
                    span { class: "badge", "Level {item.level}/{LEARN_LEVELS}" }
                }
            }
            div { class: "progress-wrap",
                div { class: "progress-header",
                    span { "Item {current_idx + 1} of {total}" }
                    span { class: "text-accent", "{progress_pct as i32}%" }
                }
                div { class: "progress-bar",
                    div { class: "progress-fill", style: "width: {progress_pct}%" }
                }
            }
            div { class: "quiz-nav",
                span { class: "quiz-nav-center", "Item {current_idx + 1} of {total}" }
            }
            div { class: "card quiz-body",
                p { class: "text-accent learn-concept", "{item.concept_title}" }
                div { class: "pre-wrap quiz-stem selectable-text", "{item.prompt}" }

                match item.modality {
                    LearnModality::MultipleChoice => rsx! {
                        for (opt_i, opt) in item.options.clone().unwrap_or_default().into_iter().enumerate() {
                            {
                                let label = opt.label.clone();
                                let is_sel = selected() == Some(label.clone());
                                let mut class = "option-btn".to_string();
                                if showing_result() {
                                    if label == item.correct_answer {
                                        class = "option-btn correct".into();
                                    } else if Some(label.clone()) == selected() {
                                        class = "option-btn incorrect".into();
                                    }
                                } else if is_sel {
                                    class = "option-btn selected".into();
                                }
                                let key_hint = (opt_i + 1).to_string();
                                rsx! {
                                    button {
                                        class: "{class}",
                                        disabled: showing_result(),
                                        onclick: move |_| selected.set(Some(label.clone())),
                                        span { class: "option-key", "{key_hint}" }
                                        span { class: "option-text", "{opt.label}.  {opt.text}" }
                                    }
                                }
                            }
                        }
                    },
                    LearnModality::Matching => rsx! {
                        div { class: "matching-grid",
                            for pair in item.matching_pairs.clone().unwrap_or_default() {
                                {
                                    let left_id = pair.left_id.clone();
                                    rsx! {
                                        div { class: "matching-row",
                                            span { class: "matching-left",
                                                strong { "{pair.left_id}. " }
                                                "{pair.left_text}"
                                            }
                                            select {
                                                class: "select matching-select",
                                                disabled: showing_result(),
                                                onchange: move |e| {
                                                    let val = e.value();
                                                    matching.with_mut(|m| {
                                                        if val.is_empty() {
                                                            m.remove(&left_id);
                                                        } else {
                                                            m.insert(left_id.clone(), val);
                                                        }
                                                    });
                                                },
                                                option { value: "", "Select match…" }
                                                for (rid, rtext) in shuffled_rights() {
                                                    option {
                                                        value: "{rid}",
                                                        selected: matching().get(&left_id) == Some(&rid),
                                                        "{rtext}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    LearnModality::ShortAnswer | LearnModality::CreateAnalogy => rsx! {
                        textarea {
                            class: "input learn-textarea",
                            rows: "5",
                            disabled: showing_result(),
                            value: "{text_answer()}",
                            placeholder: if item.modality == LearnModality::CreateAnalogy {
                                "Write your analogy here…"
                            } else {
                                "Type your answer…"
                            },
                            oninput: move |e| text_answer.set(e.value()),
                        }
                    },
                    LearnModality::AnalogyCompletion => rsx! {
                        for (opt_i, opt) in item.analogy_choices.clone().unwrap_or_default().into_iter().enumerate() {
                            {
                                let label = opt.label.clone();
                                let is_sel = selected() == Some(label.clone());
                                let mut class = "option-btn".to_string();
                                if showing_result() {
                                    if opt.text == item.correct_answer {
                                        class = "option-btn correct".into();
                                    } else if Some(label.clone()) == selected() {
                                        class = "option-btn incorrect".into();
                                    }
                                } else if is_sel {
                                    class = "option-btn selected".into();
                                }
                                let key_hint = (opt_i + 1).to_string();
                                rsx! {
                                    button {
                                        class: "{class}",
                                        disabled: showing_result(),
                                        onclick: move |_| selected.set(Some(label.clone())),
                                        span { class: "option-key", "{key_hint}" }
                                        span { class: "option-text", "{opt.label}.  {opt.text}" }
                                    }
                                }
                            }
                        }
                    },
                    LearnModality::RelationshipArrows => rsx! {
                        div { class: "arrow-grid",
                            for rel in item.relationships.clone().unwrap_or_default() {
                                {
                                    let rid = rel.id.clone();
                                    let rid_up = rid.clone();
                                    let rid_down = rid.clone();
                                    let rid_assoc = rid.clone();
                                    rsx! {
                                        div { class: "arrow-row",
                                            div { class: "arrow-pair",
                                                span { class: "arrow-anchor", "{rel.anchor}" }
                                                span { class: "arrow-connector", "→" }
                                                span { class: "arrow-target", "{rel.target}" }
                                            }
                                            div { class: "arrow-buttons",
                                                ArrowDirectionButton {
                                                    direction: ArrowDirection::Up,
                                                    active: arrows().get(&rid_up) == Some(&ArrowDirection::Up),
                                                    disabled: showing_result(),
                                                    on_pick: move |d| arrows.with_mut(|m| { m.insert(rid_up.clone(), d); }),
                                                }
                                                ArrowDirectionButton {
                                                    direction: ArrowDirection::Down,
                                                    active: arrows().get(&rid_down) == Some(&ArrowDirection::Down),
                                                    disabled: showing_result(),
                                                    on_pick: move |d| arrows.with_mut(|m| { m.insert(rid_down.clone(), d); }),
                                                }
                                                ArrowDirectionButton {
                                                    direction: ArrowDirection::Associated,
                                                    active: arrows().get(&rid_assoc) == Some(&ArrowDirection::Associated),
                                                    disabled: showing_result(),
                                                    on_pick: move |d| arrows.with_mut(|m| { m.insert(rid_assoc.clone(), d); }),
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }

                if showing_result() {
                    div {
                        class: if last_correct() {
                            "explanation-box explanation-box--success"
                        } else {
                            "explanation-box explanation-box--error"
                        },
                        p {
                            class: "explanation-status",
                            style: "font-weight: bold",
                            if last_correct() { "Correct!" } else { "Not quite — review the reference below." }
                        }
                        if leveled_up() {
                            p {
                                class: "text-accent",
                                "Level up! Next time you'll see a harder modality for this concept."
                            }
                        }
                        if !item.correct_answer.is_empty() && item.modality != LearnModality::RelationshipArrows {
                            p { "Expected: {item.correct_answer}" }
                        }
                        p { class: "text-muted", "{item.reference_explanation}" }
                    }
                }
                if matches!(
                    item.modality,
                    LearnModality::MultipleChoice | LearnModality::AnalogyCompletion
                ) {
                    p { class: "shortcut-hint",
                        "Tip: "
                        kbd { "1" }
                        "–"
                        kbd { "5" }
                        " select · "
                        kbd { "Enter" }
                        " check / next · "
                        kbd { "Esc" }
                        " exit"
                    }
                } else {
                    p { class: "shortcut-hint",
                        kbd { "Enter" }
                        " check / next · "
                        kbd { "Esc" }
                        " exit"
                    }
                }
            }
            div { class: "quiz-actions",
                div { class: "quiz-actions-right",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| advance(),
                        if !showing_result() { "Check Answer" }
                        else if index() + 1 >= items().len() { "Finish" }
                        else { "Next" }
                    }
                }
            }
        }
    }
}

#[component]
fn ArrowDirectionButton(
    direction: ArrowDirection,
    active: bool,
    disabled: bool,
    on_pick: EventHandler<ArrowDirection>,
) -> Element {
    let class = if active {
        "btn btn-primary btn-compact"
    } else {
        "btn btn-secondary btn-compact"
    };
    rsx! {
        button {
            class: "{class}",
            disabled: disabled,
            onclick: move |_| on_pick.call(direction),
            "{direction.symbol()} {direction.label()}"
        }
    }
}

fn build_learn_response(
    item: &LearnItem,
    selected: Option<String>,
    text: String,
    matching: HashMap<String, String>,
    arrows: HashMap<String, ArrowDirection>,
) -> Option<LearnResponse> {
    match item.modality {
        LearnModality::MultipleChoice | LearnModality::AnalogyCompletion => {
            selected.map(LearnResponse::SelectedOption)
        }
        LearnModality::Matching => {
            let pairs = item.matching_pairs.as_ref()?;
            if matching.len() < pairs.len() {
                return None;
            }
            Some(LearnResponse::Matching(matching))
        }
        LearnModality::ShortAnswer | LearnModality::CreateAnalogy => {
            if text.trim().is_empty() {
                None
            } else {
                Some(LearnResponse::Text(text))
            }
        }
        LearnModality::RelationshipArrows => {
            let rels = item.relationships.as_ref()?;
            if arrows.len() < rels.len() {
                return None;
            }
            Some(LearnResponse::ArrowDirections(arrows))
        }
    }
}

fn learn_response_summary(response: &LearnResponse) -> String {
    match response {
        LearnResponse::SelectedOption(s) => format!("option:{s}"),
        LearnResponse::Text(t) => format!("text:{t}"),
        LearnResponse::Matching(m) => serde_json::to_string(m).unwrap_or_default(),
        LearnResponse::ArrowDirections(a) => {
            let map: HashMap<String, String> = a
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v)))
                .collect();
            serde_json::to_string(&map).unwrap_or_default()
        }
    }
}