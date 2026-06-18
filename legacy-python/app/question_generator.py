import json
import re
import threading
from typing import Callable

from openai import OpenAI

from app.config import DIFFICULTY_DESCRIPTIONS
from app.file_parser import truncate_text
from app.nbme_guide import (
    APPROVED_LEAD_INS,
    DIFFICULTY_NBME_GUIDANCE,
    NBME_SYSTEM_RULES,
    validate_question,
)
from app.explanations import options_have_explanations
from app.source_metadata import build_citation, find_verified_quote
from app.subject_tags import normalize_subjects

GenProgressCallback = Callable[[str, float], None] | None


class QuestionGenerator:
    MAX_REGENERATION_ATTEMPTS = 2

    def __init__(self, settings: dict[str, str]):
        api_key = settings.get("api_key", "")
        if not api_key:
            raise ValueError(
                "API key not configured. Go to Settings and add your OpenAI or LM Studio API key."
            )
        self.client = OpenAI(
            api_key=api_key,
            base_url=settings.get("api_base_url", "https://api.openai.com/v1"),
        )
        self.model = settings.get("model", "gpt-4o-mini")

    def generate(
        self,
        source_text: str,
        source_file_id: str,
        quiz_settings: dict,
        filename: str = "",
        professor_name: str | None = None,
        exclude_stems: list[str] | None = None,
        on_progress: GenProgressCallback = None,
    ) -> list[dict]:
        full_text = source_text
        text = truncate_text(source_text)
        count = quiz_settings["question_count"]
        all_questions: list[dict] = []
        remaining = count
        attempt = 0
        excluded = list(exclude_stems or [])

        def report(message: str, done: int) -> None:
            if on_progress and count > 0:
                on_progress(message, done / count)

        while remaining > 0 and attempt <= self.MAX_REGENERATION_ATTEMPTS:
            if attempt > 0:
                report(f"Retrying generation ({attempt + 1})...", len(all_questions))
            raw_batch = self._call_model_with_progress(
                text,
                quiz_settings,
                remaining,
                all_questions,
                filename,
                professor_name,
                excluded,
                on_tick=lambda done: report("Calling AI model...", done),
            )
            report("Validating questions...", len(all_questions))
            for raw in raw_batch:
                try:
                    normalized = self._normalize(
                        raw,
                        source_file_id,
                        quiz_settings,
                        full_text=full_text,
                        filename=filename,
                        professor_name=professor_name,
                    )
                    issues = validate_question(
                        normalized["stem"],
                        normalized["options"],
                        normalized["correctAnswer"],
                    )
                    if issues:
                        continue
                    if not normalized.get("citation", {}).get("quote"):
                        continue
                    all_questions.append(normalized)
                    excluded.append(normalized["stem"])
                    remaining -= 1
                    report(
                        f"Generated {len(all_questions)} of {count} questions",
                        len(all_questions),
                    )
                    if remaining == 0:
                        break
                except ValueError:
                    continue
            attempt += 1

        if len(all_questions) < count:
            raise ValueError(
                f"Could only generate {len(all_questions)} of {count} NBME-compliant "
                "questions with valid source citations. Try again or reduce the question count."
            )
        return all_questions[:count]

    def _call_model_with_progress(
        self,
        source_text: str,
        settings: dict,
        count: int,
        existing: list[dict],
        filename: str,
        professor_name: str | None,
        exclude_stems: list[str],
        on_tick: Callable[[int], None],
    ) -> list[dict]:
        """Run the LLM call on a worker thread while reporting incremental progress."""
        result: list[dict] = []
        error: list[Exception] = []
        stop = threading.Event()
        base_done = len(existing)

        def worker() -> None:
            try:
                result.append(
                    self._call_model(
                        source_text,
                        settings,
                        count,
                        existing,
                        filename,
                        professor_name,
                        exclude_stems,
                    )
                )
            except Exception as exc:
                error.append(exc)
            finally:
                stop.set()

        thread = threading.Thread(target=worker, daemon=True)
        thread.start()

        on_tick(base_done)
        animated_done = base_done
        max_done = base_done + max(1, int(count * 0.9))
        step = max(1, count // 15)
        while not stop.wait(0.4):
            animated_done = min(max_done, animated_done + step)
            on_tick(animated_done)

        thread.join()
        if error:
            raise error[0]
        return result[0]

    def _call_model(
        self,
        source_text: str,
        settings: dict,
        count: int,
        existing: list[dict],
        filename: str,
        professor_name: str | None,
        exclude_stems: list[str],
    ) -> list[dict]:
        response = self.client.chat.completions.create(
            model=self.model,
            messages=[
                {"role": "system", "content": self._system_prompt(settings)},
                {
                    "role": "user",
                    "content": self._user_prompt(
                        source_text,
                        settings,
                        count,
                        existing,
                        filename,
                        professor_name,
                        exclude_stems,
                    ),
                },
            ],
            temperature=0.5,
            response_format={"type": "json_object"},
        )
        content = response.choices[0].message.content
        if not content:
            raise ValueError("No response received from the AI model.")
        try:
            parsed = json.loads(content)
        except json.JSONDecodeError as exc:
            raise ValueError("Failed to parse AI response. Please try again.") from exc
        questions = parsed.get("questions")
        if not isinstance(questions, list):
            raise ValueError("Invalid question format received from AI.")
        return questions

    def _system_prompt(self, settings: dict) -> str:
        exam = settings["exam_style"]
        exam_note = (
            "COMLEX-SPECIFIC: Integrate osteopathic principles only when supported by source material.\n"
            if exam == "COMLEX"
            else "USMLE-SPECIFIC: Mirror USMLE Step 1/Step 2 CK NBME style.\n"
        )
        lead_in_list = "\n".join(f"  - {li}" for li in APPROVED_LEAD_INS[:12])

        return f"""You are an NBME-certified item writer strictly following the NBME Item-Writing Guide (6th edition).
{exam_note}
{NBME_SYSTEM_RULES}

APPROVED LEAD-IN EXAMPLES:
{lead_in_list}

SOURCE CITATION REQUIREMENTS (MANDATORY for every item):
- Include "sourceQuote": a VERBATIM excerpt copied word-for-word from the SOURCE MATERIAL.
- The quote must contain the key concept that supports the correct answer (definition, mechanism, fact, or clinical pearl).
- Quote length: 1–3 sentences (roughly 40–300 characters). Do not paraphrase.
- If a professor/instructor name appears in the source near that content, note it in "sourceProfessor" (otherwise null).
- The quote MUST be findable as an exact substring of the provided source text.

SUBJECT TAGGING (MANDATORY for every item):
- Include "subjects": an array of 1–3 short lowercase medical subject tags for what the item assesses.
- Use common topic names such as "antibiotics", "anatomy", "cancer", "pain", "cardiology", "pharmacology".
- Tags should reflect the primary clinical/scientific domains tested, not vague labels like "medicine".

OPTION EXPLANATIONS (MANDATORY for every item):
- Every option object must include an "explanation" field.
- For the CORRECT option: explain why it is the best answer.
- For each INCORRECT option: explain specifically why it is wrong or less appropriate.
- Keep each option explanation concise (1–2 sentences).

Respond ONLY with valid JSON:
{{
  "questions": [
    {{
      "vignette": "Clinical scenario…",
      "leadIn": "Which of the following is the most likely diagnosis?",
      "options": [
        {{"label": "A", "text": "Option", "explanation": "Why A is incorrect."}},
        {{"label": "B", "text": "Option", "explanation": "Why B is the best answer."}},
        {{"label": "C", "text": "Option", "explanation": "Why C is incorrect."}},
        {{"label": "D", "text": "Option", "explanation": "Why D is incorrect."}},
        {{"label": "E", "text": "Option", "explanation": "Why E is incorrect."}}
      ],
      "correctAnswer": "B",
      "explanation": "Brief summary of the testing point and why B is best.",
      "topic": "Brief topic",
      "subjects": ["antibiotics", "infectious disease"],
      "sourceQuote": "Exact verbatim sentence(s) from the source material supporting the answer.",
      "sourceProfessor": "Dr. Jane Smith or null"
    }}
  ]
}}"""

    def _user_prompt(
        self,
        source_text: str,
        settings: dict,
        count: int,
        existing: list[dict],
        filename: str,
        professor_name: str | None,
        exclude_stems: list[str],
    ) -> str:
        difficulty = settings["difficulty"]
        nbme_diff = DIFFICULTY_NBME_GUIDANCE[difficulty]
        guide = DIFFICULTY_DESCRIPTIONS[difficulty]
        exam = settings["exam_style"]

        avoid = list(exclude_stems)
        for q in existing:
            avoid.append(q.get("topic") or q["stem"][:80])

        avoid_topics = ""
        if avoid:
            avoid_topics = "\nDO NOT REPEAT these concepts/stems:\n" + "\n".join(
                f"- {t[:100]}" for t in avoid[:30]
            )

        prof_line = (
            f"DETECTED PROFESSOR IN FILE: {professor_name}"
            if professor_name
            else "PROFESSOR: None detected in file metadata — search source text for instructor name."
        )

        return f"""Write exactly {count} NBME-compliant one-best-answer items from the source material below.

SOURCE FILE: {filename or "unknown"}
{prof_line}

EXAM STYLE: {exam}
DIFFICULTY: {difficulty.replace('_', ' ').upper()}
{guide}
{nbme_diff}

MANDATORY:
- vignette + leadIn + 5 options (A–E) + sourceQuote (verbatim from source below)
- every option must include an explanation (why correct is best; why each distractor is wrong)
- subjects: 1–3 lowercase medical subject tags (e.g. "anatomy", "cancer", "pain")
- sourceQuote must directly support the correct answer
- No duplicate concepts
{avoid_topics}

SOURCE MATERIAL:
---
{source_text}
---

Produce {count} items with verified verbatim sourceQuote for each."""

    def _normalize(
        self,
        raw: dict,
        source_file_id: str,
        settings: dict,
        full_text: str,
        filename: str,
        professor_name: str | None,
    ) -> dict:
        vignette = (raw.get("vignette") or "").strip()
        lead_in = (raw.get("leadIn") or raw.get("lead_in") or "").strip()
        legacy_stem = (raw.get("stem") or "").strip()

        if vignette and lead_in:
            stem = f"{vignette}\n\n{lead_in}"
        elif legacy_stem:
            stem = legacy_stem
        else:
            raise ValueError("Generated question is missing vignette/lead-in or stem.")

        if not raw.get("options") or not raw.get("correctAnswer") or not raw.get("explanation"):
            raise ValueError("Generated question is missing required fields.")

        labels = ["A", "B", "C", "D", "E"]
        options = []
        for i, opt in enumerate(raw["options"][:5]):
            options.append(
                {
                    "label": labels[i],
                    "text": opt["text"].strip(),
                    "explanation": (opt.get("explanation") or "").strip(),
                }
            )
        if len(options) != 5:
            raise ValueError("Generated question must have exactly 5 options.")
        if not options_have_explanations(options):
            raise ValueError("Generated question is missing per-option explanations.")

        correct = raw["correctAnswer"].strip().upper()
        if correct not in labels:
            raise ValueError(f"Invalid correct answer label: {correct}")

        stem = re.sub(r"\n{3,}", "\n\n", stem)

        raw_quote = (raw.get("sourceQuote") or raw.get("source_quote") or "").strip()
        verified_quote = find_verified_quote(raw_quote, full_text)
        if not verified_quote:
            raise ValueError("Source quote could not be verified against source material.")

        cited_professor = (
            (raw.get("sourceProfessor") or raw.get("source_professor") or "").strip() or None
        )
        if cited_professor and cited_professor.lower() in ("null", "none", "n/a"):
            cited_professor = None
        final_professor = cited_professor or professor_name

        citation = build_citation(filename, final_professor, verified_quote)
        subjects = normalize_subjects(raw.get("subjects") or raw.get("subjectTags"))
        if not subjects:
            topic_fallback = (raw.get("topic") or "").strip()
            if topic_fallback:
                subjects = normalize_subjects([topic_fallback])

        return {
            "sourceFileId": source_file_id,
            "stem": stem,
            "options": options,
            "correctAnswer": correct,
            "explanation": raw["explanation"].strip(),
            "difficulty": settings["difficulty"],
            "examStyle": settings["exam_style"],
            "topic": (raw.get("topic") or "").strip() or None,
            "subjects": subjects,
            "citation": citation,
        }