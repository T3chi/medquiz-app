use std::collections::{BTreeSet, HashMap};

use chrono::{Duration, Local, NaiveDate};

use crate::models::{DailyActivity, DashboardAnalytics, SubjectStats};

#[derive(Debug, Clone, Default)]
pub struct RawDailyUsage {
    pub date: String,
    pub app_opens: u32,
    pub questions_answered: u32,
    pub questions_correct: u32,
    pub quizzes_started: u32,
    pub quizzes_completed: u32,
}

#[derive(Debug, Clone)]
pub struct AttemptRow {
    pub answered_at: String,
    pub is_correct: bool,
    pub subjects: Vec<String>,
}

pub fn build_dashboard(
    daily_rows: &[RawDailyUsage],
    attempts: &[AttemptRow],
    total_questions: u32,
    unanswered_count: u32,
    incorrect_count: u32,
    quizzes_completed: u32,
    avg_quiz_score: f64,
) -> DashboardAnalytics {
    let today = Local::now().date_naive();
    let today_key = today.format("%Y-%m-%d").to_string();

    let mut daily_map: HashMap<String, RawDailyUsage> = HashMap::new();
    for row in daily_rows {
        daily_map.insert(row.date.clone(), row.clone());
    }

    let active_dates: BTreeSet<NaiveDate> = daily_rows
        .iter()
        .filter(|r| {
            r.app_opens > 0 || r.questions_answered > 0 || r.quizzes_completed > 0
        })
        .filter_map(|r| NaiveDate::parse_from_str(&r.date, "%Y-%m-%d").ok())
        .collect();

    let (current_streak, longest_streak) = compute_streaks(&active_dates, today);

    let today_row = daily_map.get(&today_key);
    let today_answered = today_row.map(|r| r.questions_answered).unwrap_or(0);
    let today_correct = today_row.map(|r| r.questions_correct).unwrap_or(0);
    let today_accuracy = pct(today_correct, today_answered);

    let total_attempts = attempts.len() as u32;
    let total_correct = attempts.iter().filter(|a| a.is_correct).count() as u32;
    let overall_accuracy = pct(total_correct, total_attempts);

    let (best_subjects, worst_subjects) = subject_rankings(attempts);

    let daily_activity = (0..13)
        .map(|offset| {
            let day = today - Duration::days(12 - offset);
            let key = day.format("%Y-%m-%d").to_string();
            let label = day.format("%a").to_string();
            let row = daily_map.get(&key);
            DailyActivity {
                date: key,
                label,
                answered: row.map(|r| r.questions_answered).unwrap_or(0),
                correct: row.map(|r| r.questions_correct).unwrap_or(0),
                app_opens: row.map(|r| r.app_opens).unwrap_or(0),
            }
        })
        .collect();

    DashboardAnalytics {
        current_streak,
        longest_streak,
        active_days_total: active_dates.len() as u32,
        total_questions,
        unanswered_count,
        incorrect_count,
        total_attempts,
        overall_accuracy,
        today_answered,
        today_correct,
        today_accuracy,
        quizzes_completed,
        avg_quiz_score,
        best_subjects,
        worst_subjects,
        daily_activity,
    }
}

fn pct(correct: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        correct as f64 / total as f64 * 100.0
    }
}

fn compute_streaks(active_dates: &BTreeSet<NaiveDate>, today: NaiveDate) -> (u32, u32) {
    if active_dates.is_empty() {
        return (0, 0);
    }

    let mut longest = 0_u32;
    let mut run = 0_u32;
    let mut prev: Option<NaiveDate> = None;
    for date in active_dates {
        if let Some(p) = prev {
            if *date - p == Duration::days(1) {
                run += 1;
            } else {
                run = 1;
            }
        } else {
            run = 1;
        }
        longest = longest.max(run);
        prev = Some(*date);
    }

    let mut current = 0_u32;
    let mut cursor = today;
    if !active_dates.contains(&today) {
        cursor = today - Duration::days(1);
    }
    while active_dates.contains(&cursor) {
        current += 1;
        cursor -= Duration::days(1);
    }

    (current, longest)
}

fn subject_rankings(attempts: &[AttemptRow]) -> (Vec<SubjectStats>, Vec<SubjectStats>) {
    let mut map: HashMap<String, (u32, u32)> = HashMap::new();
    for attempt in attempts {
        let tags: Vec<String> = if attempt.subjects.is_empty() {
            vec!["general".to_string()]
        } else {
            attempt.subjects.clone()
        };
        for tag in tags {
            let entry = map.entry(tag).or_insert((0, 0));
            entry.0 += 1;
            if attempt.is_correct {
                entry.1 += 1;
            }
        }
    }

    let stats: Vec<SubjectStats> = map
        .into_iter()
        .map(|(subject, (attempted, correct))| SubjectStats {
            subject,
            attempted,
            correct,
            accuracy: pct(correct, attempted),
        })
        .filter(|s| s.attempted >= 2)
        .collect();

    let mut best = stats.clone();
    best.sort_by(|a, b| {
        b.accuracy
            .partial_cmp(&a.accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.attempted.cmp(&a.attempted))
    });
    best.truncate(5);

    let mut worst = stats;
    worst.sort_by(|a, b| {
        a.accuracy
            .partial_cmp(&b.accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.attempted.cmp(&a.attempted))
    });
    worst.truncate(5);

    (best, worst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_streaks() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 18).unwrap();
        let mut dates = BTreeSet::new();
        dates.insert(today);
        dates.insert(today - Duration::days(1));
        dates.insert(today - Duration::days(2));
        let (current, longest) = compute_streaks(&dates, today);
        assert_eq!(current, 3);
        assert_eq!(longest, 3);
    }
}