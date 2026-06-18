use std::sync::{Arc, Mutex};

use rand::seq::SliceRandom;

use crate::db::Database;
use crate::llm::QuestionGenerator;
use crate::models::{AppSettings, QuizSettings};

pub struct QuizService {
    db: Arc<Mutex<Database>>,
}

impl QuizService {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    fn with_db<T>(&self, f: impl FnOnce(&Database) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let guard = self.db.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        f(&guard)
    }

    pub async fn generate_quiz(
        &self,
        source_file_id: &str,
        settings: &QuizSettings,
        app_settings: &AppSettings,
        mut on_progress: impl FnMut(&str, f64),
    ) -> anyhow::Result<Vec<String>> {
        report(&mut on_progress, "Preparing quiz...", 2.0);
        let source = self
            .with_db(|db| db.get_source_file(source_file_id))?
            .ok_or_else(|| anyhow::anyhow!("Source file not found"))?;
        let count = settings.question_count as usize;

        report(&mut on_progress, "Checking question cache...", 8.0);
        let cached = self.with_db(|db| {
            db.get_questions(
                Some(source_file_id),
                Some(&settings.difficulty),
                Some(&settings.exam_style),
            )
        })?;

        if cached.len() >= count {
            report(&mut on_progress, &format!("Using {count} cached questions"), 50.0);
            let mut rng = rand::thread_rng();
            let selected: Vec<_> = cached
                .choose_multiple(&mut rng, count)
                .cloned()
                .collect();
            report(&mut on_progress, "Quiz ready", 100.0);
            return Ok(selected.into_iter().map(|q| q.id).collect());
        }

        let mut selected = cached;
        let needed = count - selected.len();
        let cached_pct = if count > 0 {
            (selected.len() as f64 / count as f64) * 40.0
        } else {
            0.0
        };

        if !selected.is_empty() {
            report(
                &mut on_progress,
                &format!("Found {} cached — generating {needed} more with AI", selected.len()),
                12.0 + cached_pct,
            );
        } else {
            report(&mut on_progress, &format!("Generating {needed} questions with AI..."), 15.0);
        }

        let mut gen_settings = settings.clone();
        gen_settings.question_count = needed as i32;

        let generator = QuestionGenerator::new(app_settings.clone())?;
        let exclude: Vec<String> = selected.iter().map(|q| q.stem.clone()).collect();
        let generated = generator
            .generate(
                &source.text_content,
                source_file_id,
                &gen_settings,
                &source.filename,
                source.professor_name.clone(),
                &exclude,
                |msg, fraction| {
                    let pct = 15.0 + cached_pct + fraction * (73.0 - cached_pct);
                    on_progress(msg, pct);
                },
            )
            .await?;

        report(&mut on_progress, "Saving questions to database...", 92.0);
        let saved = self.with_db(|db| db.save_questions(&generated))?;
        selected.extend(saved);

        if selected.len() < count {
            anyhow::bail!(
                "Could only produce {} of {count} questions. Try again.",
                selected.len()
            );
        }

        report(&mut on_progress, "Finalizing quiz...", 98.0);
        let mut rng = rand::thread_rng();
        selected.shuffle(&mut rng);
        report(&mut on_progress, "Quiz ready", 100.0);
        Ok(selected
            .into_iter()
            .take(count)
            .map(|q| q.id)
            .collect())
    }
}

fn report(on_progress: &mut impl FnMut(&str, f64), message: &str, percent: f64) {
    on_progress(message, percent.clamp(0.0, 100.0));
}