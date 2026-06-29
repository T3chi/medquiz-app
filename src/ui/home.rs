use dioxus::prelude::*;
use keyboard_types::Key;
use rand::seq::SliceRandom;

use crate::config::{self, parse_question_count};
use crate::domain::learn::LEARN_LEVELS;
use crate::models::{DashboardAnalytics, QuizSettings, ReviewPool, UserPreferences};
use crate::ui::common::{start_review_quiz, NavView, StudyTab, with_db, use_app_state};

#[component]
pub fn Home(
    on_start: EventHandler<(Vec<String>, QuizSettings, String, NavView)>,
    on_learn_start: EventHandler<(Vec<String>, String)>,
    on_nav_materials: EventHandler<()>,
    #[allow(unused_variables)] on_show_toast: EventHandler<String>,
) -> Element {
    let state = use_app_state();
    let mut analytics = use_signal(DashboardAnalytics::default);
    let mut prefs = use_signal(UserPreferences::default);
    let mut study_tab = use_signal(|| StudyTab::Review);
    let mut review_count = use_signal(|| "10".to_string());
    let practice_count = use_signal(|| "10".to_string());
    let mut learn_count = use_signal(|| "10".to_string());
    let mut practice_settings = use_signal(QuizSettings::default);
    let filter_file = use_signal(String::new);
    let mut show_onboarding = use_signal(|| false);
    let mut onboarding_step = use_signal(|| 0u8);

    let state_for_effect = state.clone();
    use_effect(move || {
        let _ = with_db(&state_for_effect, |db| {
            analytics.set(db.get_dashboard_analytics()?);
            let p = db.get_user_preferences()?;
            prefs.set(p.clone());
            practice_settings.set(p.default_quiz_settings);
            if !db.is_onboarding_complete() {
                show_onboarding.set(true);
            }
            Ok(())
        });
    });

    let data = analytics();
    let daily_goal = prefs().daily_goal.max(1);
    let goal_pct = (data.today_answered as f64 / daily_goal as f64 * 100.0).min(100.0);
    let review_n = parse_question_count(&review_count(), 10)
        .unwrap_or(10)
        .clamp(config::MIN_QUESTION_COUNT, config::MAX_QUESTION_COUNT) as usize;

    let mut finish_onboarding = {
        let state = state.clone();
        let mut show_onboarding = show_onboarding;
        move || {
            let _ = with_db(&state, |db| db.set_onboarding_complete());
            show_onboarding.set(false);
        }
    };

    rsx! {
        if show_onboarding() {
            div {
                class: "modal-overlay",
                tabindex: "0",
                onkeydown: {
                    let state = state.clone();
                    let mut show_onboarding_key = show_onboarding;
                    move |e: Event<KeyboardData>| {
                    if e.modifiers().ctrl() || e.modifiers().alt() || e.modifiers().meta() {
                        return;
                    }
                    match e.key() {
                        Key::Escape => {
                            if onboarding_step() > 0 {
                                onboarding_step.set(onboarding_step().saturating_sub(1));
                            } else {
                                let _ = with_db(&state, |db| db.set_onboarding_complete());
                                show_onboarding_key.set(false);
                            }
                            e.prevent_default();
                        }
                        Key::Enter if onboarding_step() < 2 => {
                            onboarding_step.set(onboarding_step() + 1);
                            e.prevent_default();
                        }
                        _ => {}
                    }
                    }
                },
                div { class: "modal-card modal-card--wide",
                    if onboarding_step() == 0 {
                        h3 { "Welcome to MedQuiz" }
                        p { class: "text-secondary",
                            "Turn lecture PDFs into board-style questions, track progress, and study with quizzes and Learn mode."
                        }
                        ol { class: "onboarding-steps",
                            li { "Connect AI in Settings (OpenAI or LM Studio)" }
                            li { "Upload materials under Materials" }
                            li { "Start a session from Home" }
                        }
                    } else if onboarding_step() == 1 {
                        h3 { "Your study loop" }
                        p { class: "text-secondary",
                            "Home is your command center: review missed questions, practice with filters, or progress through Learn modalities."
                        }
                    } else {
                        h3 { "Ready to study" }
                        p { class: "text-secondary",
                            if data.total_questions == 0 {
                                "Upload a lecture under Materials to generate your first questions."
                            } else {
                                "You already have {data.total_questions} questions — start a session below."
                            }
                        }
                    }
                    div { class: "modal-actions",
                        if onboarding_step() > 0 {
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| onboarding_step.set(onboarding_step().saturating_sub(1)),
                                "Back"
                            }
                        }
                        if onboarding_step() < 2 {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| onboarding_step.set(onboarding_step() + 1),
                                "Next"
                            }
                        } else {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    finish_onboarding();
                                    if data.total_questions == 0 {
                                        on_nav_materials.call(());
                                    }
                                },
                                if data.total_questions == 0 { "Go to Materials" } else { "Get started" }
                            }
                        }
                    }
                }
            }
        }

        div { class: "page page--wide",
            h1 { class: "page-title", "Home" }

            if data.total_questions == 0 {
                div { class: "card cta-card mb-16",
                    h3 { "Get started" }
                    p { class: "text-secondary", "Upload your first lecture PDF or PowerPoint to generate practice questions." }
                    button { class: "btn btn-primary", onclick: move |_| on_nav_materials.call(()), "Go to Materials" }
                }
            } else {
                div { class: "card cta-card mb-16",
                    div { class: "cta-row",
                        div {
                            h3 {
                                if data.unanswered_count + data.incorrect_count > 0 {
                                    "Continue studying"
                                } else {
                                    "Great work today"
                                }
                            }
                            p { class: "text-secondary",
                                "{data.unanswered_count} new · {data.incorrect_count} to review · Day {data.current_streak} streak"
                            }
                        }
                        div { class: "goal-ring", title: "Daily goal: {daily_goal} questions",
                            svg { class: "goal-ring-svg", view_box: "0 0 36 36",
                                path {
                                    class: "goal-ring-bg",
                                    d: "M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831",
                                }
                                path {
                                    class: "goal-ring-fill",
                                    stroke_dasharray: "{goal_pct}, 100",
                                    d: "M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831",
                                }
                            }
                            span { class: "goal-ring-label", "{data.today_answered}/{daily_goal}" }
                        }
                    }
                }
            }

            div { class: "card mb-16",
                h3 { "Start session" }
                div { class: "tab-row",
                    for (tab, label) in [
                        (StudyTab::Review, "Smart review"),
                        (StudyTab::Practice, "Practice"),
                        (StudyTab::Learn, "Learn"),
                    ] {
                        button {
                            class: if study_tab() == tab { "tab-btn active" } else { "tab-btn" },
                            onclick: move |_| study_tab.set(tab),
                            "{label}"
                        }
                    }
                }

                match study_tab() {
                    StudyTab::Review => rsx! {
                        p { class: "text-secondary mb-12", "Target questions that need attention." }
                        div { class: "field",
                            label { class: "label", "Questions per session" }
                            input { class: "input input-narrow", value: "{review_count()}", oninput: move |e| review_count.set(e.value()) }
                        }
                        div { class: "quick-actions",
                            button {
                                class: "btn btn-primary w-full",
                                disabled: data.unanswered_count == 0,
                                onclick: {
                                    let state = state.clone();
                                    move |_| {
                                        let ids = with_db(&state, |db| db.get_review_question_ids(ReviewPool::Unanswered, review_n)).ok().unwrap_or_default();
                                        if ids.is_empty() { return; }
                                        let Ok(qs) = with_db(&state, |db| db.get_questions_by_ids(&ids)) else { return };
                                        let Some(f) = qs.first() else { return };
                                        let s = QuizSettings { question_count: ids.len() as i32, ..practice_settings() };
                                        on_start.call((ids, s, f.source_file_id.clone(), NavView::Home));
                                    }
                                },
                                "Unanswered ({data.unanswered_count})"
                            }
                            button {
                                class: "btn btn-secondary w-full",
                                disabled: data.incorrect_count == 0,
                                onclick: {
                                    let state = state.clone();
                                    move |_| {
                                        let h = EventHandler::new(move |(ids, s, src): (Vec<String>, QuizSettings, String)| {
                                            on_start.call((ids, s, src, NavView::Home));
                                        });
                                        start_review_quiz(&state, ReviewPool::Incorrect, review_n, &h);
                                    }
                                },
                                "Incorrect ({data.incorrect_count})"
                            }
                            button {
                                class: "btn btn-secondary w-full",
                                disabled: data.unanswered_count == 0 && data.incorrect_count == 0,
                                onclick: {
                                    let state = state.clone();
                                    move |_| {
                                        let h = EventHandler::new(move |(ids, s, src): (Vec<String>, QuizSettings, String)| {
                                            on_start.call((ids, s, src, NavView::Home));
                                        });
                                        start_review_quiz(&state, ReviewPool::Mixed, review_n, &h);
                                    }
                                },
                                "Mixed ({data.unanswered_count + data.incorrect_count})"
                            }
                        }
                    },
                    StudyTab::Practice => rsx! {
                        p { class: "text-secondary mb-12", "Custom quiz from your bank with filters." }
                        PracticePanel {
                            count_input: practice_count,
                            settings: practice_settings,
                            filter_file,
                            on_start: move |(ids, s, src)| on_start.call((ids, s, src, NavView::Home)),
                        }
                    },
                    StudyTab::Learn => rsx! {
                        p { class: "text-secondary mb-12",
                            "Progress through {LEARN_LEVELS} modalities per concept. Two correct answers advance difficulty."
                        }
                        div { class: "field",
                            label { class: "label", "Concepts per session" }
                            input { class: "input input-narrow", value: "{learn_count()}", oninput: move |e| learn_count.set(e.value()) }
                        }
                        button {
                            class: "btn btn-primary w-full",
                            disabled: data.total_questions == 0,
                            onclick: {
                                let state = state.clone();
                                move |_| {
                                let count = parse_question_count(&learn_count(), 10).unwrap_or(10) as usize;
                                let Ok(ids) = with_db(&state, |db| db.get_learn_question_ids(None, count)) else { return };
                                if ids.is_empty() { return; }
                                let Ok(qs) = with_db(&state, |db| db.get_questions_by_ids(&ids)) else { return };
                                let src = qs.first().map(|q| q.source_file_id.clone()).unwrap_or_default();
                                on_learn_start.call((ids, src));
                            }
                            },
                            "Start Learn Session"
                        }
                    },
                }
            }

            div { class: "stat-grid",
                div { class: "stat-card stat-card--accent",
                    span { class: "stat-label", "Current Streak" }
                    span { class: "stat-value", "{data.current_streak}" }
                    span { class: "stat-sub", "best {data.longest_streak} days" }
                }
                div { class: "stat-card",
                    span { class: "stat-label", "Accuracy" }
                    span { class: "stat-value", "{data.overall_accuracy:.0}%" }
                    span { class: "stat-sub", "{data.total_attempts} attempts" }
                }
                div { class: "stat-card",
                    span { class: "stat-label", "Question Bank" }
                    span { class: "stat-value", "{data.total_questions}" }
                    span { class: "stat-sub", "{data.unanswered_count} new" }
                }
                div { class: "stat-card",
                    span { class: "stat-label", "Quizzes Done" }
                    span { class: "stat-value", "{data.quizzes_completed}" }
                    span { class: "stat-sub", "avg {data.avg_quiz_score:.0}%" }
                }
            }

            div { class: "card mt-16",
                h3 { "Daily Activity" }
                div { class: "activity-chart",
                    for day in data.daily_activity.iter() {
                        {
                            let height = if day.answered == 0 { 4.0 } else {
                                (day.answered as f64 / data.daily_activity.iter().map(|d| d.answered).max().unwrap_or(1).max(1) as f64 * 100.0).max(12.0)
                            };
                            let acc = if day.answered > 0 { (day.correct as f64 / day.answered as f64 * 100.0) as i32 } else { 0 };
                            rsx! {
                                div { class: "activity-col",
                                    span { class: "activity-value", "{day.answered}" }
                                    div { class: "activity-bar", style: "height: {height}%", title: "{day.date}: {day.answered} answered ({acc}% correct)" }
                                    span { class: "activity-label", "{day.label}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "grid-2 mt-16",
                div { class: "card",
                    h3 { "Strongest Subjects" }
                    for subj in data.best_subjects.iter() {
                        button {
                            class: "subject-row subject-row--clickable",
                            onclick: {
                                let state = state.clone();
                                let subject = subj.subject.clone();
                                move |_| {
                                    let Ok(ids) = with_db(&state, |db| db.get_question_ids_by_subject(&subject, 10)) else { return };
                                    if ids.is_empty() { return; }
                                    let Ok(qs) = with_db(&state, |db| db.get_questions_by_ids(&ids)) else { return };
                                    let Some(f) = qs.first() else { return };
                                    let s = QuizSettings { question_count: ids.len() as i32, ..practice_settings() };
                                    on_start.call((ids, s, f.source_file_id.clone(), NavView::Home));
                                }
                            },
                            span { class: "subject-name", "{subj.subject}" }
                            span { class: "text-success", "{subj.accuracy:.0}%" }
                            span { class: "text-muted", "{subj.correct}/{subj.attempted}" }
                        }
                    }
                    if data.best_subjects.is_empty() {
                        p { class: "text-muted", "Answer more to see rankings." }
                    }
                }
                div { class: "card",
                    h3 { "Needs Work" }
                    for subj in data.worst_subjects.iter() {
                        button {
                            class: "subject-row subject-row--clickable",
                            onclick: {
                                let state = state.clone();
                                let subject = subj.subject.clone();
                                move |_| {
                                    let Ok(ids) = with_db(&state, |db| db.get_question_ids_by_subject(&subject, 10)) else { return };
                                    if ids.is_empty() { return; }
                                    let Ok(qs) = with_db(&state, |db| db.get_questions_by_ids(&ids)) else { return };
                                    let Some(f) = qs.first() else { return };
                                    let s = QuizSettings { question_count: ids.len() as i32, ..practice_settings() };
                                    on_start.call((ids, s, f.source_file_id.clone(), NavView::Home));
                                }
                            },
                            span { class: "subject-name", "{subj.subject}" }
                            span { class: "text-error", "{subj.accuracy:.0}%" }
                            span { class: "text-muted", "{subj.correct}/{subj.attempted}" }
                        }
                    }
                    if data.worst_subjects.is_empty() {
                        p { class: "text-muted", "Answer more to identify weak areas." }
                    }
                }
            }
        }
    }
}

#[component]
fn PracticePanel(
    count_input: Signal<String>,
    settings: Signal<QuizSettings>,
    filter_file: Signal<String>,
    on_start: EventHandler<(Vec<String>, QuizSettings, String)>,
) -> Element {
    let state = use_app_state();
    let mut questions = use_signal(Vec::new);
    let state_eff = state.clone();
    use_effect(move || {
        if let Ok(qs) = with_db(&state_eff, |db| db.get_questions(None, None, None)) {
            questions.set(qs);
        }
    });
    let available = questions().iter().filter(|q| {
        if !filter_file().is_empty() && q.source_file_id != filter_file() { return false; }
        q.difficulty == settings().difficulty && q.exam_style == settings().exam_style
    }).count();
    let count = parse_question_count(&count_input(), 10).unwrap_or(10).min(available.max(1) as i32);

    rsx! {
        div { class: "form-grid",
            div { class: "field",
                label { class: "label", "Questions" }
                input { class: "input input-narrow", value: "{count_input()}", oninput: move |e| count_input.set(e.value()) }
                p { class: "text-accent", "{count} of {available} available" }
            }
            div { class: "field",
                label { class: "label", "Exam style" }
                select {
                    class: "select",
                    onchange: move |e| settings.with_mut(|s| s.exam_style = e.value()),
                    option { value: "USMLE", selected: settings().exam_style == "USMLE", "USMLE" }
                    option { value: "COMLEX", selected: settings().exam_style == "COMLEX", "COMLEX" }
                }
            }
            div { class: "field",
                label { class: "label", "Difficulty" }
                select {
                    class: "select",
                    onchange: move |e| settings.with_mut(|s| s.difficulty = e.value()),
                    option { value: "definition", "Definition" }
                    option { value: "first_order", "First Order" }
                    option { value: "second_order", "Second Order" }
                }
            }
        }
        button {
            class: "btn btn-primary w-full",
            disabled: available == 0,
            onclick: move |_| {
                let mut list: Vec<_> = questions().into_iter().filter(|q| {
                    if !filter_file().is_empty() && q.source_file_id != filter_file() { return false; }
                    q.difficulty == settings().difficulty && q.exam_style == settings().exam_style
                }).collect();
                list.shuffle(&mut rand::thread_rng());
                let selected: Vec<_> = list.into_iter().take(count as usize).collect();
                if selected.is_empty() { return; }
                let ids: Vec<String> = selected.iter().map(|q| q.id.clone()).collect();
                let src = selected[0].source_file_id.clone();
                let mut s = settings();
                s.question_count = ids.len() as i32;
                on_start.call((ids, s, src));
            },
            "Start Practice Quiz"
        }
    }
}