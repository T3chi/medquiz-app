"""Orchestrates quiz generation with DB caching to minimize LLM calls."""

import random
from typing import Callable

from app.database import Database
from app.question_generator import QuestionGenerator


ProgressCallback = Callable[[str, float], None] | None


class QuizService:
    def __init__(self, db: Database):
        self.db = db

    def _report(self, on_progress: ProgressCallback, message: str, percent: float) -> None:
        if on_progress:
            on_progress(message, min(100.0, max(0.0, percent)))

    def generate_quiz(
        self,
        source_file_id: str,
        settings: dict,
        app_settings: dict[str, str],
        on_progress: ProgressCallback = None,
    ) -> list[str]:
        """
        Return question IDs for a quiz. Reuses matching cached questions from the DB
        when possible; only calls the LLM for the shortfall.
        """
        self._report(on_progress, "Preparing quiz...", 2)

        source = self.db.get_source_file(source_file_id)
        if not source:
            raise ValueError("Source file not found")

        count = settings["question_count"]
        difficulty = settings["difficulty"]
        exam_style = settings["exam_style"]

        self._report(on_progress, "Checking question cache...", 8)

        cached = self.db.get_questions(
            source_file_id=source_file_id,
            difficulty=difficulty,
            exam_style=exam_style,
        )

        if len(cached) >= count:
            self._report(on_progress, f"Using {count} cached questions", 50)
            selected = random.sample(cached, count)
            self._report(on_progress, "Quiz ready", 100)
            return [q["id"] for q in selected]

        selected: list[dict] = list(cached)
        needed = count - len(selected)
        cached_pct = (len(cached) / count) * 40 if count else 0

        if cached:
            self._report(
                on_progress,
                f"Found {len(cached)} cached — generating {needed} more with AI",
                12 + cached_pct,
            )
        else:
            self._report(on_progress, f"Generating {needed} questions with AI...", 15)

        text = source["textContent"]
        if not text:
            raise ValueError("Source file has no text content")

        gen_settings = {**settings, "question_count": needed}

        def on_gen_progress(message: str, fraction: float) -> None:
            # AI generation spans 15%–88% of total progress
            pct = 15 + cached_pct + fraction * (73 - cached_pct)
            self._report(on_progress, message, pct)

        generator = QuestionGenerator(app_settings)
        generated = generator.generate(
            source_text=text,
            source_file_id=source_file_id,
            quiz_settings=gen_settings,
            filename=source["filename"],
            professor_name=source.get("professorName"),
            exclude_stems=[q["stem"] for q in selected],
            on_progress=on_gen_progress,
        )

        self._report(on_progress, "Saving questions to database...", 92)

        saved = self.db.save_questions(generated)
        selected.extend(saved)

        if len(selected) < count:
            raise ValueError(
                f"Could only produce {len(selected)} of {count} questions. Try again."
            )

        self._report(on_progress, "Finalizing quiz...", 98)
        random.shuffle(selected)
        self._report(on_progress, "Quiz ready", 100)
        return [q["id"] for q in selected[:count]]