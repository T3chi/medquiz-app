use std::sync::{Arc, Mutex};

use dioxus::prelude::*;
use rand::seq::SliceRandom;

use crate::config::{self, parse_question_count, LAST_UPLOAD_DIR_KEY};
use crate::db::{copy_upload, Database};
use crate::domain::citations::format_citation_display;
use crate::domain::explanations::format_option_explanations;
use crate::models::{
    difficulty_label, AppSettings, DashboardAnalytics, Question, QuizSettings, ReviewPool,
    SourceFile, DIFFICULTY_DESCRIPTIONS, DIFFICULTY_LABELS,
};
use crate::parsers::parse_file;
use crate::services::QuizService;
use crate::AppState;

#[derive(Clone, PartialEq)]
enum NavView {
    Dashboard,
    Create,
    TakeQuiz,
    Bank,
    Settings,
}

#[derive(Clone, PartialEq)]
enum Screen {
    Nav(NavView),
    Quiz {
        question_ids: Vec<String>,
        settings: QuizSettings,
        source_file_id: String,
        session_id: String,
    },
}

#[component]
pub fn App() -> Element {
    let state = use_hook(|| {
        let db = Database::open().expect("Failed to open database");
        AppState {
            db: Arc::new(Mutex::new(db)),
        }
    });
    use_context_provider(|| state.clone());

    let mut screen = use_signal(|| Screen::Nav(NavView::Dashboard));
    let mut bank_refresh = use_signal(|| 0u32);
    let mut files_refresh = use_signal(|| 0u32);
    let mut dashboard_refresh = use_signal(|| 0u32);

    let state_for_open = state.clone();
    use_effect(move || {
        let _ = with_db(&state_for_open, |db| db.record_app_open());
    });

    let on_quiz_ready = {
        let state = state.clone();
        let mut screen = screen;
        move |(ids, settings, source_id): (Vec<String>, QuizSettings, String)| {
            if let Ok(session) = with_db(&state, |db| db.create_quiz_session(&source_id, &settings, &ids)) {
                screen.set(Screen::Quiz {
                    question_ids: ids,
                    settings,
                    source_file_id: source_id,
                    session_id: session.id,
                });
            }
        }
    };

    let on_take_start = {
        let state = state.clone();
        let mut screen = screen;
        move |(ids, settings, source_id): (Vec<String>, QuizSettings, String)| {
            if let Ok(session) = with_db(&state, |db| db.create_quiz_session(&source_id, &settings, &ids)) {
                screen.set(Screen::Quiz {
                    question_ids: ids,
                    settings,
                    source_file_id: source_id,
                    session_id: session.id,
                });
            }
        }
    };

    let on_quiz_exit = {
        let mut screen = screen;
        let mut bank_refresh = bank_refresh;
        let mut files_refresh = files_refresh;
        let mut dashboard_refresh = dashboard_refresh;
        move |_| {
            bank_refresh.set(bank_refresh() + 1);
            files_refresh.set(files_refresh() + 1);
            dashboard_refresh.set(dashboard_refresh() + 1);
            screen.set(Screen::Nav(NavView::Dashboard));
        }
    };

    rsx! {
        document::Style { {include_str!("../assets/style.css")} }
        div { class: "app-shell",
            Sidebar {
                current: match screen() {
                    Screen::Nav(v) => v,
                    Screen::Quiz { .. } => NavView::Dashboard,
                },
                quiz_active: matches!(screen(), Screen::Quiz { .. }),
                on_nav: move |v| screen.set(Screen::Nav(v)),
            }
            div { class: "content",
                match screen() {
                    Screen::Nav(NavView::Dashboard) => rsx! {
                        Dashboard {
                            key: "{dashboard_refresh()}",
                            on_start: on_take_start,
                        }
                    },
                    Screen::Nav(NavView::Create) => rsx! {
                        CreateQuiz {
                            key: "{files_refresh()}",
                            on_quiz_ready,
                        }
                    },
                    Screen::Nav(NavView::TakeQuiz) => rsx! {
                        TakeQuiz { on_start: on_take_start }
                    },
                    Screen::Nav(NavView::Bank) => rsx! {
                        QuestionBank { key: "{bank_refresh()}" }
                    },
                    Screen::Nav(NavView::Settings) => rsx! {
                        SettingsView {}
                    },
                    Screen::Quiz { question_ids, settings, source_file_id, session_id } => rsx! {
                        QuizView {
                            question_ids,
                            settings,
                            source_file_id,
                            session_id,
                            on_exit: on_quiz_exit,
                        }
                    },
                }
            }
        }
    }
}

fn with_db<T>(state: &AppState, f: impl FnOnce(&Database) -> anyhow::Result<T>) -> anyhow::Result<T> {
    let guard = state.db.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
    f(&guard)
}

fn use_app_state() -> AppState {
    use_context::<AppState>()
}

fn start_review_quiz(
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
fn Dashboard(on_start: EventHandler<(Vec<String>, QuizSettings, String)>) -> Element {
    let state = use_app_state();
    let mut analytics = use_signal(DashboardAnalytics::default);
    let mut review_count = use_signal(|| "10".to_string());

    let state_for_effect = state.clone();
    use_effect(move || {
        if let Ok(data) = with_db(&state_for_effect, |db| db.get_dashboard_analytics()) {
            analytics.set(data);
        }
    });

    let data = analytics();
    let review_n = parse_question_count(&review_count(), 10)
        .unwrap_or(10)
        .clamp(config::MIN_QUESTION_COUNT, config::MAX_QUESTION_COUNT) as usize;

    let activity_max = data
        .daily_activity
        .iter()
        .map(|d| d.answered)
        .max()
        .unwrap_or(1)
        .max(1);

    rsx! {
        div { class: "page page--wide",
            h1 { class: "page-title", "Dashboard" }
            p { class: "page-subtitle",
                "Track your progress, streaks, and weak areas — then jump straight into targeted review."
            }

            div { class: "stat-grid",
                div { class: "stat-card stat-card--accent",
                    span { class: "stat-label", "Current Streak" }
                    span { class: "stat-value", "{data.current_streak}" }
                    span { class: "stat-sub", "days active · best {data.longest_streak}" }
                }
                div { class: "stat-card",
                    span { class: "stat-label", "Overall Accuracy" }
                    span { class: "stat-value", "{data.overall_accuracy:.0}%" }
                    span { class: "stat-sub", "{data.total_attempts} lifetime attempts" }
                }
                div { class: "stat-card",
                    span { class: "stat-label", "Today" }
                    span { class: "stat-value", "{data.today_answered}" }
                    span { class: "stat-sub",
                        if data.today_answered > 0 {
                            "{data.today_accuracy:.0}% correct today"
                        } else {
                            "No questions answered yet"
                        }
                    }
                }
                div { class: "stat-card",
                    span { class: "stat-label", "Question Bank" }
                    span { class: "stat-value", "{data.total_questions}" }
                    span { class: "stat-sub",
                        "{data.unanswered_count} new · {data.incorrect_count} to review"
                    }
                }
            }

            div { class: "grid-2",
                div { class: "card",
                    h3 { "Quick Review" }
                    p { class: "text-secondary", "Build a quiz from questions that need your attention." }
                    div { class: "field",
                        label { class: "label", "Questions per quiz" }
                        input {
                            class: "input input-narrow",
                            value: "{review_count()}",
                            oninput: move |e| review_count.set(e.value()),
                        }
                    }
                    div { class: "quick-actions",
                        button {
                            class: "btn btn-primary w-full",
                            disabled: data.unanswered_count == 0,
                            onclick: {
                                let state = state.clone();
                                move |_| start_review_quiz(
                                    &state,
                                    ReviewPool::Unanswered,
                                    review_n,
                                    &on_start,
                                )
                            },
                            "Unanswered ({data.unanswered_count})"
                        }
                        button {
                            class: "btn btn-secondary w-full",
                            disabled: data.incorrect_count == 0,
                            onclick: {
                                let state = state.clone();
                                move |_| start_review_quiz(
                                    &state,
                                    ReviewPool::Incorrect,
                                    review_n,
                                    &on_start,
                                )
                            },
                            "Incorrect ({data.incorrect_count})"
                        }
                        button {
                            class: "btn btn-secondary w-full",
                            disabled: data.unanswered_count == 0 && data.incorrect_count == 0,
                            onclick: {
                                let state = state.clone();
                                move |_| start_review_quiz(
                                    &state,
                                    ReviewPool::Mixed,
                                    review_n,
                                    &on_start,
                                )
                            },
                            "Mixed Review ({data.unanswered_count + data.incorrect_count})"
                        }
                    }
                }
                div { class: "card",
                    h3 { "Quiz Performance" }
                    div { class: "metric-list",
                        div { class: "metric-row",
                            span { class: "text-secondary", "Quizzes completed" }
                            span { class: "metric-value", "{data.quizzes_completed}" }
                        }
                        div { class: "metric-row",
                            span { class: "text-secondary", "Average quiz score" }
                            span { class: "metric-value", "{data.avg_quiz_score:.0}%" }
                        }
                        div { class: "metric-row",
                            span { class: "text-secondary", "Active study days" }
                            span { class: "metric-value", "{data.active_days_total}" }
                        }
                        div { class: "metric-row",
                            span { class: "text-secondary", "Lifetime attempts" }
                            span { class: "metric-value", "{data.total_attempts}" }
                        }
                    }
                }
            }

            div { class: "card mt-16",
                h3 { "Daily Activity" }
                p { class: "text-muted mb-12", "Questions answered over the last 14 days" }
                div { class: "activity-chart",
                    for day in data.daily_activity.iter() {
                        {
                            let height = if day.answered == 0 {
                                4.0
                            } else {
                                (day.answered as f64 / activity_max as f64 * 100.0).max(12.0)
                            };
                            let title = format!(
                                "{}: {} answered ({}% correct)",
                                day.date,
                                day.answered,
                                if day.answered > 0 {
                                    (day.correct as f64 / day.answered as f64 * 100.0) as i32
                                } else {
                                    0
                                }
                            );
                            rsx! {
                                div { class: "activity-col", title: "{title}",
                                    div {
                                        class: "activity-bar",
                                        style: "height: {height}%",
                                    }
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
                    if data.best_subjects.is_empty() {
                        p { class: "text-muted", "Answer more questions to see subject rankings." }
                    }
                    for subj in data.best_subjects.iter() {
                        div { class: "subject-row",
                            span { class: "subject-name", "{subj.subject}" }
                            span { class: "text-success", "{subj.accuracy:.0}%" }
                            span { class: "text-muted", "{subj.correct}/{subj.attempted}" }
                        }
                    }
                }
                div { class: "card",
                    h3 { "Needs Work" }
                    if data.worst_subjects.is_empty() {
                        p { class: "text-muted", "Answer more questions to identify weak areas." }
                    }
                    for subj in data.worst_subjects.iter() {
                        div { class: "subject-row",
                            span { class: "subject-name", "{subj.subject}" }
                            span { class: "text-error", "{subj.accuracy:.0}%" }
                            span { class: "text-muted", "{subj.correct}/{subj.attempted}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Sidebar(current: NavView, quiz_active: bool, on_nav: EventHandler<NavView>) -> Element {
    rsx! {
        div { class: "sidebar",
            p { class: "logo-title", "⚕ MedQuiz" }
            p { class: "logo-sub", "USMLE & COMLEX Prep" }
            for (view, label) in [
                (NavView::Dashboard, "Dashboard"),
                (NavView::Create, "Create Quiz"),
                (NavView::TakeQuiz, "Take Quiz"),
                (NavView::Bank, "Question Bank"),
                (NavView::Settings, "Settings"),
            ] {
                button {
                    class: if current == view { "nav-btn active" } else { "nav-btn" },
                    disabled: quiz_active,
                    onclick: move |_| on_nav.call(view.clone()),
                    "{label}"
                }
            }
        }
    }
}

#[component]
fn CreateQuiz(on_quiz_ready: EventHandler<(Vec<String>, QuizSettings, String)>) -> Element {
    let state = use_app_state();
    let mut files = use_signal(|| Vec::<SourceFile>::new());
    let mut selected = use_signal(|| None::<String>);
    let mut settings = use_signal(QuizSettings::default);
    let mut count_input = use_signal(|| "10".to_string());
    let mut generating = use_signal(|| false);
    let mut progress_msg = use_signal(String::new);
    let mut progress_pct = use_signal(|| 0.0);
    let mut error_msg = use_signal(String::new);

    let state_for_effect = state.clone();
    use_effect(move || {
        if let Ok(list) = with_db(&state_for_effect, |db| db.get_source_files()) {
            files.set(list);
            if selected().is_none() {
                if let Some(f) = files().first() {
                    selected.set(Some(f.id.clone()));
                }
            }
        }
    });

    let state_for_upload = state.clone();
    let state_for_generate = state.clone();

    rsx! {
        div { class: "page page--wide",
            h1 { class: "page-title", "Create Quiz" }
            p { class: "page-subtitle", "Upload lecture slides or notes and generate board-style practice questions." }
            if !error_msg().is_empty() {
                p { class: "text-error", "{error_msg()}" }
            }
            div { class: "grid-2",
                div { class: "card card-fill",
                    h3 { "Study Materials" }
                    button {
                        class: "btn btn-secondary w-full",
                        style: "margin: 12px 0;",
                        disabled: generating(),
                        onclick: move |_| {
                            let state = state_for_upload.clone();
                            let mut files = files;
                            let mut selected = selected;
                            let mut error_msg = error_msg;
                            spawn(async move {
                                let last_dir = with_db(&state, |db| db.get_preference(LAST_UPLOAD_DIR_KEY)).ok();
                                let mut dialog = rfd::FileDialog::new()
                                    .add_filter("Study Materials", &["pdf", "pptx", "ppt"]);
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
                        },
                        "Upload PDF or PowerPoint"
                    }
                    div { class: "scroll-list",
                        for file in files() {
                            {
                                let id = file.id.clone();
                                let delete_id = id.clone();
                                let state_for_delete = state.clone();
                                let is_sel = selected().as_deref() == Some(&id);
                                rsx! {
                                    div {
                                        key: "{file.id}",
                                        class: if is_sel { "file-row selected" } else { "file-row" },
                                        onclick: move |_| selected.set(Some(id.clone())),
                                        span { "[{file.file_type.to_uppercase()}] {file.filename}" }
                                        span { class: "text-muted",
                                            "{file.text_length / 1000}k chars"
                                        }
                                        button {
                                            class: "btn btn-ghost ml-auto",
                                            onclick: move |e| {
                                                e.stop_propagation();
                                                let state = state_for_delete.clone();
                                                let mut files = files;
                                                let fid = delete_id.clone();
                                                if let Ok(()) = with_db(&state, |db| db.delete_source_file(&fid)) {
                                                    if selected().as_deref() == Some(&fid) { selected.set(None); }
                                                    if let Ok(list) = with_db(&state, |db| db.get_source_files()) {
                                                        files.set(list);
                                                    }
                                                }
                                            },
                                            "X"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "card",
                    h3 { "Quiz Settings" }
                    div { class: "field",
                        label { class: "label", "Number of Questions" }
                        div { class: "radio-row",
                            input {
                                class: "input input-narrow",
                                value: "{count_input()}",
                                disabled: generating(),
                                oninput: move |e| count_input.set(e.value()),
                            }
                            span { class: "text-muted", "questions ({config::MIN_QUESTION_COUNT}–{config::MAX_QUESTION_COUNT})" }
                        }
                    }
                    div { class: "field",
                        label { class: "label", "Exam Style" }
                        div { class: "radio-row",
                            for style in ["USMLE", "COMLEX"] {
                                label { class: "radio-label",
                                    input {
                                        r#type: "radio",
                                        name: "exam",
                                        checked: settings().exam_style == style,
                                        onchange: move |_| settings.with_mut(|s| s.exam_style = style.to_string()),
                                    }
                                    "{style}"
                                }
                            }
                        }
                    }
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
                                            name: "difficulty",
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
                        label { class: "label", "Show Answers" }
                        div { class: "radio-row",
                            label { class: "radio-label",
                                input {
                                    r#type: "radio",
                                    checked: settings().answer_timing == "per_question",
                                    onchange: move |_| settings.with_mut(|s| s.answer_timing = "per_question".into()),
                                }
                                "After Each Question"
                            }
                            label { class: "radio-label",
                                input {
                                    r#type: "radio",
                                    checked: settings().answer_timing == "end_of_quiz",
                                    onchange: move |_| settings.with_mut(|s| s.answer_timing = "end_of_quiz".into()),
                                }
                                "End of Quiz"
                            }
                        }
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
                    button {
                        class: "btn btn-primary w-full",
                        disabled: generating() || selected().is_none(),
                        onclick: move |_| {
                            let count = parse_question_count(&count_input(), settings().question_count);
                            let Some(count) = count else {
                                error_msg.set(format!("Enter a number between {} and {}", config::MIN_QUESTION_COUNT, config::MAX_QUESTION_COUNT));
                                return;
                            };
                            error_msg.set(String::new());
                            let mut s = settings();
                            s.question_count = count;
                            settings.set(s);
                            count_input.set(count.to_string());

                            let Some(source_id) = selected() else { return };
                            let state = state_for_generate.clone();
                            let s = settings();
                            generating.set(true);
                            progress_msg.set("Starting...".into());
                            progress_pct.set(0.0);
                            spawn(async move {
                                let app_settings = with_db(&state, |db| db.get_app_settings()).unwrap_or_default();
                                let service = QuizService::new(state.db.clone());
                                let result = service.generate_quiz(
                                    &source_id,
                                    &s,
                                    &app_settings,
                                    |msg, pct| {
                                        progress_msg.set(msg.to_string());
                                        progress_pct.set(pct);
                                    },
                                ).await;
                                generating.set(false);
                                match result {
                                    Ok(ids) => on_quiz_ready.call((ids, s, source_id)),
                                    Err(e) => error_msg.set(e.to_string()),
                                }
                            });
                        },
                        if generating() { "Generating..." } else { "Generate Quiz" }
                    }
                }
            }
        }
    }
}

#[component]
fn TakeQuiz(on_start: EventHandler<(Vec<String>, QuizSettings, String)>) -> Element {
    let state = use_app_state();
    let mut questions = use_signal(|| Vec::<Question>::new());
    let mut files = use_signal(|| Vec::<SourceFile>::new());
    let mut settings = use_signal(QuizSettings::default);
    let mut count_input = use_signal(|| "10".to_string());
    let mut filter_file = use_signal(String::new);

    let state_for_effect = state.clone();
    use_effect(move || {
        let _ = with_db(&state_for_effect, |db| {
            questions.set(db.get_questions(None, None, None)?);
            files.set(db.get_source_files()?);
            Ok(())
        });
    });

    let filtered_count = || {
        questions().into_iter().filter(|q| {
            if !filter_file().is_empty() && q.source_file_id != filter_file() { return false; }
            q.difficulty == settings().difficulty && q.exam_style == settings().exam_style
        }).count()
    };

    let available = filtered_count();
    let count = parse_question_count(&count_input(), 10).unwrap_or(10).min(available as i32);

    if questions().is_empty() {
        return rsx! {
            div { class: "page page--compact",
                h1 { class: "page-title", "Take Quiz" }
                div { class: "empty-state", "No questions available. Create a quiz first." }
            }
        };
    }

    rsx! {
        div { class: "page page--compact",
            h1 { class: "page-title", "Take Quiz" }
            p { class: "page-subtitle", "Practice with questions from your bank." }
            div { class: "card",
                div { class: "form-grid",
                    div { class: "field field-full",
                        label { class: "label", "Number of Questions" }
                        input { class: "input input-narrow", value: "{count_input()}", oninput: move |e| count_input.set(e.value()) }
                        p { class: "text-accent", "{count} of {available} matching questions available" }
                    }
                    div { class: "field",
                        label { class: "label", "Filter by Source" }
                        select {
                            class: "select",
                            onchange: move |e| filter_file.set(e.value()),
                            option { value: "", "All sources" }
                            for f in files() {
                                option { value: "{f.id}", "{f.filename}" }
                            }
                        }
                    }
                    div { class: "field",
                        label { class: "label", "Exam Style" }
                        select {
                            class: "select",
                            onchange: move |e| settings.with_mut(|s| s.exam_style = e.value()),
                            option { value: "USMLE", "USMLE" }
                            option { value: "COMLEX", "COMLEX" }
                        }
                    }
                    div { class: "field field-full",
                        label { class: "label", "Difficulty" }
                        select {
                            class: "select",
                            onchange: move |e| settings.with_mut(|s| s.difficulty = e.value()),
                            for (k, l) in DIFFICULTY_LABELS {
                                option { value: "{k}", selected: settings().difficulty == *k, "{l}" }
                            }
                        }
                    }
                }
                button {
                    class: "btn btn-primary w-full",
                    disabled: available == 0,
                    onclick: move |_| {
                        let mut list: Vec<Question> = questions().into_iter().filter(|q| {
                            if !filter_file().is_empty() && q.source_file_id != filter_file() { return false; }
                            q.difficulty == settings().difficulty && q.exam_style == settings().exam_style
                        }).collect();
                        list.shuffle(&mut rand::thread_rng());
                        let take = count as usize;
                        let selected: Vec<_> = list.into_iter().take(take).collect();
                        if selected.is_empty() { return; }
                        let ids: Vec<String> = selected.iter().map(|q| q.id.clone()).collect();
                        let source_id = selected[0].source_file_id.clone();
                        on_start.call((ids, settings(), source_id));
                    },
                    "Start Quiz"
                }
            }
        }
    }
}

#[component]
fn QuestionBank() -> Element {
    let state = use_app_state();
    let mut all_questions = use_signal(|| Vec::<Question>::new());
    let mut search = use_signal(String::new);
    let mut expanded = use_signal(|| None::<String>);

    let state_for_effect = state.clone();
    use_effect(move || {
        if let Ok(qs) = with_db(&state_for_effect, |db| db.get_questions(None, None, None)) {
            all_questions.set(qs);
        }
    });

    let query = search().trim().to_lowercase();
    let questions: Vec<Question> = if query.is_empty() {
        all_questions()
    } else {
        all_questions().into_iter().filter(|q| {
            q.subjects.iter().any(|t| t.to_lowercase().contains(&query))
                || q.topic.as_ref().map(|t| t.to_lowercase().contains(&query)).unwrap_or(false)
        }).collect()
    };

    let mut tags: Vec<String> = Vec::new();
    for q in all_questions().iter() {
        for t in &q.subjects {
            if !tags.contains(t) { tags.push(t.clone()); }
        }
    }
    tags.sort();

    rsx! {
        div { class: "page page--wide",
            h1 { class: "page-title", "Question Bank" }
            p { class: "page-subtitle",
                if query.is_empty() {
                    "{all_questions().len()} questions stored locally"
                } else {
                    "{questions.len()} of {all_questions().len()} matching \"{search()}\""
                }
            }
            div { class: "search-row mb-12",
                input {
                    class: "input",
                    placeholder: "Search by subject tag (e.g. antibiotics, anatomy, pain)",
                    value: "{search()}",
                    oninput: move |e| search.set(e.value()),
                }
                button { class: "btn btn-secondary", onclick: move |_| search.set(String::new()), "Clear" }
            }
            if !tags.is_empty() {
                div { class: "chip-row mb-16",
                    span { class: "text-muted", "Tags:" }
                    for tag in tags.iter().take(20) {
                        {
                            let t = tag.clone();
                            let active = query == t.to_lowercase();
                            rsx! {
                                button {
                                    class: if active { "tag active" } else { "tag" },
                                    onclick: move |_| search.set(t.clone()),
                                    "{tag}"
                                }
                            }
                        }
                    }
                }
            }
            if questions.is_empty() {
                div { class: "empty-state", "No questions match your search." }
            }
            div { class: "question-list",
            for (i, q) in questions.clone().into_iter().enumerate() {
                {
                    let qid = q.id.clone();
                    let delete_qid = qid.clone();
                    let state_for_delete = state.clone();
                    let stem_preview = {
                        let short: String = q.stem.chars().take(200).collect();
                        if q.stem.len() > 200 {
                            format!("{short}...")
                        } else {
                            short
                        }
                    };
                    let is_exp = expanded() == Some(qid.clone());
                    rsx! {
                        div {
                            class: if is_exp { "question-card expanded" } else { "question-card" },
                            key: "{q.id}",
                            div { class: "flex-between",
                                span { class: "text-muted", "#{questions.len() - i}" }
                                span { class: "text-secondary", "{q.exam_style} · {difficulty_label(&q.difficulty)}" }
                                span {
                                    class: match q.last_result.as_deref() {
                                        Some("correct") => "text-success",
                                        Some("incorrect") => "text-error",
                                        _ => "text-muted",
                                    },
                                    if q.attempt_count > 0 {
                                        "Last: {q.last_result.clone().unwrap_or_else(|| \"unanswered\".into())} ({q.attempt_count} tries)"
                                    } else {
                                        "Unanswered"
                                    }
                                }
                                button {
                                    class: "btn btn-ghost ml-auto",
                                    onclick: move |_| {
                                        let _ = with_db(&state_for_delete, |db| db.delete_question(&delete_qid));
                                        if let Ok(qs) = with_db(&state_for_delete, |db| db.get_questions(None, None, None)) {
                                            all_questions.set(qs);
                                        }
                                        if expanded() == Some(delete_qid.clone()) { expanded.set(None); }
                                    },
                                    "Delete"
                                }
                            }
                            div { class: "chip-row",
                                for tag in q.subjects.clone() {
                                    button { class: "tag", onclick: move |_| search.set(tag.clone()), "{tag}" }
                                }
                            }
                            button {
                                class: "stem-preview",
                                onclick: move |_| {
                                    if expanded() == Some(qid.clone()) { expanded.set(None); }
                                    else { expanded.set(Some(qid.clone())); }
                                },
                                "{stem_preview}"
                            }
                            if is_exp {
                                div { class: "detail-panel",
                                    for opt in q.options.iter() {
                                        {
                                            let mark = if opt.label == q.correct_answer { " ✓" } else { "" };
                                            let opt_class = if opt.label == q.correct_answer { "text-success" } else { "text-secondary" };
                                            rsx! {
                                                div { class: "detail-option",
                                                    p { class: "{opt_class}", "{opt.label}. {opt.text}{mark}" }
                                                    if !opt.explanation.is_empty() {
                                                        p { class: "text-muted detail-option-explanation", "{opt.explanation}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if !q.explanation.is_empty() {
                                        p { class: "text-muted", "Summary: {q.explanation}" }
                                    }
                                    {
                                        let cite = format_citation_display(&q.citation);
                                        if !cite.is_empty() {
                                            rsx! { p { class: "text-accent pre-wrap", "{cite}" } }
                                        } else { rsx! {} }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            }
        }
    }
}

#[component]
fn SettingsView() -> Element {
    let state = use_app_state();
    let mut settings = use_signal(|| AppSettings::default());
    let mut status = use_signal(String::new);

    let state_for_effect = state.clone();
    use_effect(move || {
        if let Ok(s) = with_db(&state_for_effect, |db| db.get_app_settings()) {
            settings.set(s);
        }
    });

    rsx! {
        div { class: "page page--compact",
            h1 { class: "page-title", "Settings" }
            p { class: "page-subtitle", "Configure your AI provider for question generation." }
            div { class: "card",
                div { class: "radio-row mb-16",
                    button { class: "btn btn-secondary", onclick: move |_| {
                        settings.with_mut(|s| {
                            s.api_base_url = "https://api.openai.com/v1".into();
                            s.model = "gpt-4o-mini".into();
                        });
                    }, "OpenAI Preset" }
                    button { class: "btn btn-secondary", onclick: move |_| {
                        settings.with_mut(|s| {
                            s.api_base_url = "http://localhost:1234/v1".into();
                            s.model = "local-model".into();
                        });
                    }, "LM Studio Preset" }
                }
                div { class: "field",
                    label { class: "label", "API Key" }
                    input {
                        class: "input",
                        r#type: "password",
                        value: "{settings().api_key}",
                        oninput: move |e| settings.with_mut(|s| s.api_key = e.value()),
                    }
                }
                div { class: "field",
                    label { class: "label", "API Base URL" }
                    input { class: "input", value: "{settings().api_base_url}", oninput: move |e| settings.with_mut(|s| s.api_base_url = e.value()) }
                }
                div { class: "field",
                    label { class: "label", "Model" }
                    input { class: "input", value: "{settings().model}", oninput: move |e| settings.with_mut(|s| s.model = e.value()) }
                }
                if !status().is_empty() {
                    p { class: "text-success", "{status()}" }
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let s = settings();
                        if with_db(&state, |db| db.save_app_settings(&s)).is_ok() {
                            status.set("Settings saved successfully".into());
                        }
                    },
                    "Save Settings"
                }
            }
            div { class: "card mt-16",
                p { class: "text-secondary",
                    "MedQuiz stores all questions locally in SQLite at your app data folder.\nDefinition · First Order · Second Order difficulty levels supported."
                }
            }
        }
    }
}

#[component]
fn QuizView(
    question_ids: Vec<String>,
    settings: QuizSettings,
    source_file_id: String,
    session_id: String,
    on_exit: EventHandler<()>,
) -> Element {
    let state = use_app_state();
    let questions = use_signal(|| {
        with_db(&state, |db| db.get_questions_by_ids(&question_ids)).unwrap_or_default()
    });
    let mut index = use_signal(|| 0usize);
    let mut answers = use_signal(|| std::collections::HashMap::<String, String>::new());
    let mut selected = use_signal(|| None::<String>);
    let mut showing_result = use_signal(|| false);
    let mut complete = use_signal(|| false);

    if complete() {
        let total = questions().len();
        let correct_count = questions().iter().filter(|q| answers().get(&q.id).map(|a| a == &q.correct_answer).unwrap_or(false)).count();
        let score = if total > 0 { correct_count as f64 / total as f64 * 100.0 } else { 0.0 };
        let _ = with_db(&state, |db| db.complete_quiz_session(&session_id, score));

        return rsx! {
            div { class: "page page--quiz",
                h1 { class: "page-title", "Quiz Complete" }
                p {
                    class: if score >= 70.0 { "text-success" } else { "text-error" },
                    class: "score-display",
                    "{score:.0}%  ({correct_count}/{total} correct)"
                }
                div { class: "results-scroll",
                    for (i, q) in questions().iter().enumerate() {
                        {
                            let ok = answers().get(&q.id).map(|a| a == &q.correct_answer).unwrap_or(false);
                            let body = format_option_explanations(q);
                            let cite = format_citation_display(&q.citation);
                            let full = if cite.is_empty() { body } else { format!("{body}\n\n{cite}") };
                            let border_color = if ok { "var(--success)" } else { "var(--error)" };
                            let status = if ok { "Correct" } else { "Incorrect" };
                            let status_class = if ok { "text-success" } else { "text-error" };
                            rsx! {
                                div {
                                    class: "question-card",
                                    style: "border-color: {border_color}",
                                    p { class: "{status_class}", "Q{i+1} — {status}" }
                                    p { "{q.stem}" }
                                    p { class: "text-secondary", "Your answer: {answers().get(&q.id).cloned().unwrap_or_else(|| \"—\".into())} | Correct: {q.correct_answer}" }
                                    p { class: "text-muted pre-wrap", "{full}" }
                                }
                            }
                        }
                    }
                }
                button { class: "btn btn-primary", onclick: move |_| on_exit.call(()), "Back to Home" }
            }
        };
    }

    let q = questions().get(index()).cloned();
    let Some(q) = q else {
        return rsx! { div { class: "page page--quiz", "No questions loaded." } };
    };

    let per_q = settings.answer_timing == "per_question";
    let total = questions().len();
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

    rsx! {
        div { class: "page page--quiz",
            div { class: "quiz-header",
                button { class: "btn btn-ghost", onclick: move |_| on_exit.call(()), "← Exit Quiz" }
                div {
                    span { class: "badge", "{settings.exam_style}" }
                    span { class: "badge", "{difficulty_label(&settings.difficulty)}" }
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
                if !tags.is_empty() { p { class: "text-accent", "{tags}" } }
                div { class: "pre-wrap quiz-stem selectable-text", "{q.stem}" }
                for opt in q.options.iter() {
                    {
                        let label = opt.label.clone();
                        let is_sel = selected() == Some(label.clone());
                        let mut class = "option-btn".to_string();
                        if showing_result() {
                            if label == q.correct_answer { class = "option-btn correct".into(); }
                            else if Some(&label) == selected().as_ref() && label != q.correct_answer { class = "option-btn incorrect".into(); }
                        } else if is_sel {
                            class = "option-btn selected".into();
                        }
                        rsx! {
                            button {
                                class: "{class}",
                                disabled: showing_result() && per_q,
                                onclick: move |_| selected.set(Some(label.clone())),
                                "  {opt.label}.  {opt.text}"
                            }
                        }
                    }
                }
                if showing_result() && per_q {
                    {
                        let ok = selected() == Some(q.correct_answer.clone());
                        let body = format_option_explanations(&q);
                        let cite = format_citation_display(&q.citation);
                        let full = if cite.is_empty() { body } else { format!("{body}\n\n{cite}") };
                        let expl_color = if ok { "var(--success)" } else { "var(--error)" };
                        rsx! {
                            div {
                                class: "explanation-box",
                                style: "color: {expl_color}",
                                p { style: "font-weight:bold",
                                    if ok { "Correct!" } else { "Incorrect — Answer: {q.correct_answer}" }
                                }
                                p { "{full}" }
                            }
                        }
                    }
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
                    onclick: move |_| {
                        if !showing_result() {
                            let Some(sel) = selected() else { return };
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
                    },
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