use std::path::Path;

use chrono::{Local, Utc};
use rand::seq::SliceRandom;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::config::db_path;
use crate::domain::analytics::{build_dashboard, AttemptRow, RawDailyUsage};
use crate::domain::backfill::{enrich_citation, infer_citation, infer_subjects};
use crate::domain::citations::extract_professor_name;
use crate::models::{
    AppSettings, Citation, DashboardAnalytics, Question, QuestionAttempt, QuestionOption,
    QuizSession, QuizSettings, ReviewPool, SourceFile, SourceFileDetail,
};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> anyhow::Result<Self> {
        let path = db_path();
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS source_files (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_type TEXT NOT NULL,
                text_content TEXT NOT NULL,
                uploaded_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS questions (
                id TEXT PRIMARY KEY,
                source_file_id TEXT NOT NULL,
                stem TEXT NOT NULL,
                options TEXT NOT NULL,
                correct_answer TEXT NOT NULL,
                explanation TEXT NOT NULL,
                difficulty TEXT NOT NULL,
                exam_style TEXT NOT NULL,
                topic TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (source_file_id) REFERENCES source_files(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS quiz_sessions (
                id TEXT PRIMARY KEY,
                source_file_id TEXT NOT NULL,
                settings TEXT NOT NULL,
                question_ids TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                score REAL,
                FOREIGN KEY (source_file_id) REFERENCES source_files(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS question_attempts (
                id TEXT PRIMARY KEY,
                question_id TEXT NOT NULL,
                quiz_session_id TEXT,
                selected_answer TEXT NOT NULL,
                is_correct INTEGER NOT NULL,
                answered_at TEXT NOT NULL,
                FOREIGN KEY (question_id) REFERENCES questions(id) ON DELETE CASCADE,
                FOREIGN KEY (quiz_session_id) REFERENCES quiz_sessions(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS daily_usage (
                date TEXT PRIMARY KEY,
                app_opens INTEGER NOT NULL DEFAULT 0,
                questions_answered INTEGER NOT NULL DEFAULT 0,
                questions_correct INTEGER NOT NULL DEFAULT 0,
                quizzes_started INTEGER NOT NULL DEFAULT 0,
                quizzes_completed INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;
        self.migrate()?;
        self.backfill_daily_usage_from_history()?;
        Ok(())
    }

    fn today_key() -> String {
        Local::now().format("%Y-%m-%d").to_string()
    }

    fn bump_daily(&self, field: &str, delta: i32) -> anyhow::Result<()> {
        let today = Self::today_key();
        self.conn.execute(
            &format!(
                "INSERT INTO daily_usage (date, {field}) VALUES (?1, ?2)
                 ON CONFLICT(date) DO UPDATE SET {field} = {field} + ?2"
            ),
            params![today, delta],
        )?;
        Ok(())
    }

    pub fn record_app_open(&self) -> anyhow::Result<()> {
        self.bump_daily("app_opens", 1)
    }

    fn backfill_daily_usage_from_history(&self) -> anyhow::Result<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM daily_usage", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(());
        }

        self.conn.execute(
            "INSERT INTO daily_usage (date, questions_answered, questions_correct)
             SELECT substr(answered_at, 1, 10), COUNT(*), SUM(is_correct)
             FROM question_attempts
             GROUP BY substr(answered_at, 1, 10)",
            [],
        )?;

        self.conn.execute(
            "INSERT INTO daily_usage (date, quizzes_completed)
             SELECT substr(completed_at, 1, 10), COUNT(*)
             FROM quiz_sessions
             WHERE completed_at IS NOT NULL
             GROUP BY substr(completed_at, 1, 10)
             ON CONFLICT(date) DO UPDATE SET
               quizzes_completed = quizzes_completed + excluded.quizzes_completed",
            [],
        )?;

        Ok(())
    }

    fn migrate(&self) -> anyhow::Result<()> {
        let cols = |table: &str| -> anyhow::Result<Vec<String>> {
            let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
            Ok(rows.filter_map(Result::ok).collect())
        };
        for col in [
            ("source_files", "professor_name"),
            ("questions", "citation_filename"),
            ("questions", "citation_professor"),
            ("questions", "citation_quote"),
            ("questions", "subjects"),
            ("questions", "last_result"),
            ("questions", "last_answered_at"),
        ] {
            if !cols(col.0)?.contains(&col.1.to_string()) {
                self.conn.execute(
                    &format!("ALTER TABLE {} ADD COLUMN {} TEXT", col.0, col.1),
                    [],
                )?;
            }
        }
        Ok(())
    }

    pub fn save_source_file(
        &self,
        filename: &str,
        file_path: &str,
        file_type: &str,
        text_content: &str,
    ) -> anyhow::Result<SourceFile> {
        let id = Uuid::new_v4().to_string();
        let uploaded_at = Utc::now().to_rfc3339();
        let professor = extract_professor_name(text_content);
        self.conn.execute(
            "INSERT INTO source_files (id, filename, file_path, file_type, text_content, uploaded_at, professor_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, filename, file_path, file_type, text_content, uploaded_at, professor],
        )?;
        Ok(SourceFile {
            id,
            filename: filename.to_string(),
            file_path: file_path.to_string(),
            file_type: file_type.to_string(),
            uploaded_at,
            text_length: text_content.len(),
            professor_name: professor,
        })
    }

    pub fn get_source_files(&self) -> anyhow::Result<Vec<SourceFile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, file_path, file_type, uploaded_at, LENGTH(text_content)
             FROM source_files ORDER BY uploaded_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SourceFile {
                id: r.get(0)?,
                filename: r.get(1)?,
                file_path: r.get(2)?,
                file_type: r.get(3)?,
                uploaded_at: r.get(4)?,
                text_length: r.get::<_, i64>(5)? as usize,
                professor_name: None,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn get_source_file(&self, id: &str) -> anyhow::Result<Option<SourceFileDetail>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, file_path, file_type, text_content, uploaded_at, professor_name
             FROM source_files WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |r| {
            Ok(SourceFileDetail {
                id: r.get(0)?,
                filename: r.get(1)?,
                file_path: r.get(2)?,
                file_type: r.get(3)?,
                text_content: r.get(4)?,
                uploaded_at: r.get(5)?,
                professor_name: r.get(6)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn delete_source_file(&self, id: &str) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM source_files WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn save_questions(&self, questions: &[Question]) -> anyhow::Result<Vec<Question>> {
        let mut saved = Vec::new();
        for q in questions {
            let id = Uuid::new_v4().to_string();
            let created_at = Utc::now().to_rfc3339();
            let citation = q.citation.as_ref();
            let subjects = serde_json::to_string(&q.subjects)?;
            self.conn.execute(
                "INSERT INTO questions (id, source_file_id, stem, options, correct_answer, explanation,
                 difficulty, exam_style, topic, created_at, citation_filename, citation_professor,
                 citation_quote, subjects)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    id,
                    q.source_file_id,
                    q.stem,
                    serde_json::to_string(&q.options)?,
                    q.correct_answer,
                    q.explanation,
                    q.difficulty,
                    q.exam_style,
                    q.topic,
                    created_at,
                    citation.and_then(|c| c.filename.clone()),
                    citation.and_then(|c| c.professor_name.clone()),
                    citation.and_then(|c| c.quote.clone()),
                    subjects,
                ],
            )?;
            let mut item = q.clone();
            item.id = id;
            item.created_at = Some(created_at);
            saved.push(item);
        }
        Ok(saved)
    }

    pub fn get_questions(
        &self,
        source_file_id: Option<&str>,
        difficulty: Option<&str>,
        exam_style: Option<&str>,
    ) -> anyhow::Result<Vec<Question>> {
        let mut sql = String::from(
            "SELECT q.*, (SELECT COUNT(*) FROM question_attempts a WHERE a.question_id = q.id) AS attempt_count
             FROM questions q WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();
        if let Some(v) = source_file_id {
            sql.push_str(" AND q.source_file_id = ?");
            binds.push(v.to_string());
        }
        if let Some(v) = difficulty {
            sql.push_str(" AND q.difficulty = ?");
            binds.push(v.to_string());
        }
        if let Some(v) = exam_style {
            sql.push_str(" AND q.exam_style = ?");
            binds.push(v.to_string());
        }
        sql.push_str(" ORDER BY q.created_at DESC");
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), row_to_question)?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn get_questions_by_ids(&self, ids: &[String]) -> anyhow::Result<Vec<Question>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT q.*, (SELECT COUNT(*) FROM question_attempts a WHERE a.question_id = q.id) AS attempt_count
             FROM questions q WHERE q.id IN ({placeholders})"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let map: std::collections::HashMap<String, Question> = stmt
            .query_map(params.as_slice(), row_to_question)?
            .filter_map(Result::ok)
            .map(|q| (q.id.clone(), q))
            .collect();
        Ok(ids.iter().filter_map(|id| map.get(id).cloned()).collect())
    }

    pub fn delete_question(&self, id: &str) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM questions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn record_question_attempt(
        &self,
        question_id: &str,
        selected_answer: &str,
        is_correct: bool,
        quiz_session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_string();
        let answered_at = Utc::now().to_rfc3339();
        let last_result = if is_correct { "correct" } else { "incorrect" };
        self.conn.execute(
            "INSERT INTO question_attempts (id, question_id, quiz_session_id, selected_answer, is_correct, answered_at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                id,
                question_id,
                quiz_session_id,
                selected_answer,
                is_correct as i32,
                answered_at
            ],
        )?;
        self.conn.execute(
            "UPDATE questions SET last_result = ?1, last_answered_at = ?2 WHERE id = ?3",
            params![last_result, answered_at, question_id],
        )?;
        self.bump_daily("questions_answered", 1)?;
        if is_correct {
            self.bump_daily("questions_correct", 1)?;
        }
        Ok(())
    }

    pub fn get_question_attempts(&self, question_id: &str) -> anyhow::Result<Vec<QuestionAttempt>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, question_id, selected_answer, is_correct, answered_at
             FROM question_attempts WHERE question_id = ?1 ORDER BY answered_at DESC",
        )?;
        let rows = stmt.query_map(params![question_id], |r| {
            Ok(QuestionAttempt {
                id: r.get(0)?,
                question_id: r.get(1)?,
                selected_answer: r.get(2)?,
                is_correct: r.get::<_, i32>(3)? != 0,
                answered_at: r.get(4)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn create_quiz_session(
        &self,
        source_file_id: &str,
        settings: &QuizSettings,
        question_ids: &[String],
    ) -> anyhow::Result<QuizSession> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO quiz_sessions (id, source_file_id, settings, question_ids, started_at)
             VALUES (?1,?2,?3,?4,?5)",
            params![
                id,
                source_file_id,
                serde_json::to_string(settings)?,
                serde_json::to_string(question_ids)?,
                Utc::now().to_rfc3339()
            ],
        )?;
        self.bump_daily("quizzes_started", 1)?;
        Ok(QuizSession {
            id,
            source_file_id: source_file_id.to_string(),
            settings: settings.clone(),
            question_ids: question_ids.to_vec(),
        })
    }

    pub fn complete_quiz_session(&self, session_id: &str, score: f64) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE quiz_sessions SET completed_at = ?1, score = ?2 WHERE id = ?3",
            params![Utc::now().to_rfc3339(), score, session_id],
        )?;
        self.bump_daily("quizzes_completed", 1)?;
        Ok(())
    }

    pub fn get_dashboard_analytics(&self) -> anyhow::Result<DashboardAnalytics> {
        let daily_rows = self.get_daily_usage_rows()?;
        let attempts = self.get_attempt_rows_for_analytics()?;
        let questions = self.get_questions(None, None, None)?;

        let total_questions = questions.len() as u32;
        let unanswered_count = questions
            .iter()
            .filter(|q| q.attempt_count == 0)
            .count() as u32;
        let incorrect_count = questions
            .iter()
            .filter(|q| q.last_result.as_deref() == Some("incorrect"))
            .count() as u32;

        let (quizzes_completed, avg_quiz_score) = self.conn.query_row(
            "SELECT COUNT(*), AVG(score) FROM quiz_sessions WHERE completed_at IS NOT NULL",
            [],
            |r| Ok((r.get::<_, i64>(0)? as u32, r.get::<_, Option<f64>>(1)?.unwrap_or(0.0))),
        )?;

        Ok(build_dashboard(
            &daily_rows,
            &attempts,
            total_questions,
            unanswered_count,
            incorrect_count,
            quizzes_completed,
            avg_quiz_score,
        ))
    }

    fn get_daily_usage_rows(&self) -> anyhow::Result<Vec<RawDailyUsage>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, app_opens, questions_answered, questions_correct,
                    quizzes_started, quizzes_completed
             FROM daily_usage ORDER BY date ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(RawDailyUsage {
                date: r.get(0)?,
                app_opens: r.get::<_, i64>(1)? as u32,
                questions_answered: r.get::<_, i64>(2)? as u32,
                questions_correct: r.get::<_, i64>(3)? as u32,
                quizzes_started: r.get::<_, i64>(4)? as u32,
                quizzes_completed: r.get::<_, i64>(5)? as u32,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn get_attempt_rows_for_analytics(&self) -> anyhow::Result<Vec<AttemptRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.answered_at, a.is_correct, q.subjects
             FROM question_attempts a
             JOIN questions q ON q.id = a.question_id",
        )?;
        let rows = stmt.query_map([], |r| {
            let subjects_json: Option<String> = r.get(2)?;
            let subjects: Vec<String> = subjects_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            Ok(AttemptRow {
                answered_at: r.get(0)?,
                is_correct: r.get::<_, i32>(1)? != 0,
                subjects,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn get_review_question_ids(&self, pool: ReviewPool, limit: usize) -> anyhow::Result<Vec<String>> {
        let questions = self.get_questions(None, None, None)?;
        let mut ids: Vec<String> = questions
            .into_iter()
            .filter(|q| match pool {
                ReviewPool::Unanswered => q.attempt_count == 0,
                ReviewPool::Incorrect => q.last_result.as_deref() == Some("incorrect"),
                ReviewPool::Mixed => {
                    q.attempt_count == 0 || q.last_result.as_deref() == Some("incorrect")
                }
            })
            .map(|q| q.id)
            .collect();

        ids.shuffle(&mut rand::thread_rng());
        ids.truncate(limit);
        Ok(ids)
    }

    pub fn get_app_settings(&self) -> anyhow::Result<AppSettings> {
        let mut settings = AppSettings::default();
        let mut stmt = self.conn.prepare("SELECT key, value FROM app_settings")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows.flatten() {
            match row.0.as_str() {
                "apiKey" => settings.api_key = row.1,
                "apiBaseUrl" => settings.api_base_url = row.1,
                "model" => settings.model = row.1,
                _ => {}
            }
        }
        Ok(settings)
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> anyhow::Result<()> {
        for (k, v) in [
            ("apiKey", &settings.api_key),
            ("apiBaseUrl", &settings.api_base_url),
            ("model", &settings.model),
        ] {
            self.conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
                params![k, v],
            )?;
        }
        Ok(())
    }

    pub fn get_preference(&self, key: &str) -> anyhow::Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |r| r.get(0))?;
        Ok(rows.next().transpose()?.unwrap_or_default())
    }

    pub fn set_preference(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn update_question_metadata(
        &self,
        question_id: &str,
        subjects: &[String],
        citation: Option<&Citation>,
    ) -> anyhow::Result<()> {
        let subjects_json = serde_json::to_string(subjects)?;
        let (cite_file, cite_prof, cite_quote) = match citation {
            Some(c) => (
                c.filename.clone(),
                c.professor_name.clone(),
                c.quote.clone(),
            ),
            None => (None, None, None),
        };
        self.conn.execute(
            "UPDATE questions SET subjects = ?1, citation_filename = ?2,
             citation_professor = ?3, citation_quote = ?4 WHERE id = ?5",
            params![subjects_json, cite_file, cite_prof, cite_quote, question_id],
        )?;
        Ok(())
    }

    pub fn backfill_question_metadata(&self) -> anyhow::Result<BackfillReport> {
        let questions = self.get_questions(None, None, None)?;
        let mut report = BackfillReport::default();

        for question in questions {
            let needs_subjects = question.subjects.is_empty();
            let needs_citation = question
                .citation
                .as_ref()
                .and_then(|c| c.quote.as_ref())
                .is_none_or(|q| q.trim().is_empty());
            let needs_cite_enrichment = question.citation.as_ref().is_some_and(|c| {
                c.filename.as_ref().is_none_or(|f| f.is_empty())
                    || c.professor_name.as_ref().is_none_or(|p| p.is_empty())
            });

            if !needs_subjects && !needs_citation && !needs_cite_enrichment {
                continue;
            }

            let Some(source) = self.get_source_file(&question.source_file_id)? else {
                report.skipped += 1;
                continue;
            };

            let mut subjects = question.subjects.clone();
            if needs_subjects {
                subjects = infer_subjects(question.topic.as_deref(), &source.filename);
                if !subjects.is_empty() {
                    report.subjects_added += 1;
                }
            }

            let mut citation = question.citation.clone();
            if needs_citation {
                citation = infer_citation(&question, &source);
                if citation.is_some() {
                    report.citations_added += 1;
                } else {
                    report.citations_missing += 1;
                }
            }

            if let Some(ref mut cite) = citation {
                enrich_citation(cite, &source);
                if needs_cite_enrichment && !needs_citation {
                    report.citations_enriched += 1;
                }
            }

            if needs_subjects || needs_citation || needs_cite_enrichment {
                self.update_question_metadata(
                    &question.id,
                    &subjects,
                    citation.as_ref(),
                )?;
                report.updated += 1;
            }
        }

        Ok(report)
    }
}

#[derive(Debug, Default)]
pub struct BackfillReport {
    pub updated: usize,
    pub subjects_added: usize,
    pub citations_added: usize,
    pub citations_enriched: usize,
    pub citations_missing: usize,
    pub skipped: usize,
}

fn row_to_question(row: &rusqlite::Row<'_>) -> rusqlite::Result<Question> {
    let options_json: String = row.get("options")?;
    let options: Vec<QuestionOption> = serde_json::from_str(&options_json).unwrap_or_default();
    let subjects_json: Option<String> = row.get("subjects").ok();
    let subjects: Vec<String> = subjects_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let citation = row.get::<_, Option<String>>("citation_quote").ok().flatten();
    let citation = citation.map(|quote| Citation {
        filename: row.get("citation_filename").ok(),
        professor_name: row.get("citation_professor").ok(),
        quote: Some(quote),
    });
    Ok(Question {
        id: row.get("id")?,
        source_file_id: row.get("source_file_id")?,
        stem: row.get("stem")?,
        options,
        correct_answer: row.get("correct_answer")?,
        explanation: row.get("explanation")?,
        difficulty: row.get("difficulty")?,
        exam_style: row.get("exam_style")?,
        topic: row.get("topic").ok(),
        subjects,
        citation,
        last_result: row.get("last_result").ok(),
        last_answered_at: row.get("last_answered_at").ok(),
        attempt_count: row.get::<_, i64>("attempt_count").unwrap_or(0) as i32,
        created_at: row.get("created_at").ok(),
    })
}

pub fn copy_upload(src: &Path, dest_dir: &Path) -> anyhow::Result<std::path::PathBuf> {
    let name = src
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
    let ts = Utc::now().timestamp();
    let dest = dest_dir.join(format!("{ts}_{}", name.to_string_lossy()));
    std::fs::copy(src, &dest)?;
    Ok(dest)
}