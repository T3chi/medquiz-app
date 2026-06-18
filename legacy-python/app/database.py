import json
import sqlite3
import uuid
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from app.config import get_db_path
from app.source_metadata import extract_professor_name


class Database:
    def __init__(self, db_path: Path | None = None):
        self.db_path = db_path or get_db_path()
        self._init_schema()

    @contextmanager
    def _conn(self):
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA journal_mode = WAL")
        conn.execute("PRAGMA foreign_keys = ON")
        try:
            yield conn
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            conn.close()

    def _init_schema(self) -> None:
        with self._conn() as conn:
            conn.executescript(
                """
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
                """
            )
            self._migrate_schema(conn)

    def _migrate_schema(self, conn: sqlite3.Connection) -> None:
        sf_cols = {row[1] for row in conn.execute("PRAGMA table_info(source_files)")}
        if "professor_name" not in sf_cols:
            conn.execute("ALTER TABLE source_files ADD COLUMN professor_name TEXT")

        q_cols = {row[1] for row in conn.execute("PRAGMA table_info(questions)")}
        if "citation_filename" not in q_cols:
            conn.execute("ALTER TABLE questions ADD COLUMN citation_filename TEXT")
        if "citation_professor" not in q_cols:
            conn.execute("ALTER TABLE questions ADD COLUMN citation_professor TEXT")
        if "citation_quote" not in q_cols:
            conn.execute("ALTER TABLE questions ADD COLUMN citation_quote TEXT")
        if "subjects" not in q_cols:
            conn.execute("ALTER TABLE questions ADD COLUMN subjects TEXT")
        if "last_result" not in q_cols:
            conn.execute("ALTER TABLE questions ADD COLUMN last_result TEXT")
        if "last_answered_at" not in q_cols:
            conn.execute("ALTER TABLE questions ADD COLUMN last_answered_at TEXT")

    def save_source_file(
        self, filename: str, file_path: str, file_type: str, text_content: str
    ) -> dict[str, Any]:
        file_id = str(uuid.uuid4())
        uploaded_at = datetime.now(timezone.utc).isoformat()
        professor_name = extract_professor_name(text_content)
        with self._conn() as conn:
            conn.execute(
                """INSERT INTO source_files
                   (id, filename, file_path, file_type, text_content, uploaded_at, professor_name)
                   VALUES (?, ?, ?, ?, ?, ?, ?)""",
                (file_id, filename, file_path, file_type, text_content, uploaded_at, professor_name),
            )
        return {
            "id": file_id,
            "filename": filename,
            "filePath": file_path,
            "fileType": file_type,
            "uploadedAt": uploaded_at,
            "textLength": len(text_content),
            "professorName": professor_name,
        }

    def get_source_files(self) -> list[dict[str, Any]]:
        with self._conn() as conn:
            rows = conn.execute(
                """SELECT id, filename, file_path, file_type, uploaded_at,
                          LENGTH(text_content) as text_length
                   FROM source_files ORDER BY uploaded_at DESC"""
            ).fetchall()
        return [
            {
                "id": r["id"],
                "filename": r["filename"],
                "filePath": r["file_path"],
                "fileType": r["file_type"],
                "uploadedAt": r["uploaded_at"],
                "textLength": r["text_length"],
            }
            for r in rows
        ]

    def get_source_file_text(self, file_id: str) -> str | None:
        source = self.get_source_file(file_id)
        return source["textContent"] if source else None

    def get_source_file(self, file_id: str) -> dict[str, Any] | None:
        with self._conn() as conn:
            row = conn.execute(
                """SELECT id, filename, file_path, file_type, text_content,
                          uploaded_at, professor_name
                   FROM source_files WHERE id = ?""",
                (file_id,),
            ).fetchone()
        if not row:
            return None
        return {
            "id": row["id"],
            "filename": row["filename"],
            "filePath": row["file_path"],
            "fileType": row["file_type"],
            "textContent": row["text_content"],
            "uploadedAt": row["uploaded_at"],
            "professorName": row["professor_name"],
            "textLength": len(row["text_content"]),
        }

    def delete_source_file(self, file_id: str) -> None:
        with self._conn() as conn:
            conn.execute("DELETE FROM source_files WHERE id = ?", (file_id,))

    def save_questions(self, questions: list[dict[str, Any]]) -> list[dict[str, Any]]:
        saved = []
        with self._conn() as conn:
            for q in questions:
                qid = str(uuid.uuid4())
                created_at = datetime.now(timezone.utc).isoformat()
                citation = q.get("citation") or {}
                conn.execute(
                    """INSERT INTO questions
                       (id, source_file_id, stem, options, correct_answer, explanation,
                        difficulty, exam_style, topic, created_at,
                        citation_filename, citation_professor, citation_quote, subjects)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                    (
                        qid,
                        q["sourceFileId"],
                        q["stem"],
                        json.dumps(q["options"]),
                        q["correctAnswer"],
                        q["explanation"],
                        q["difficulty"],
                        q["examStyle"],
                        q.get("topic"),
                        created_at,
                        citation.get("filename"),
                        citation.get("professorName"),
                        citation.get("quote"),
                        json.dumps(q.get("subjects") or []),
                    ),
                )
                saved.append({**q, "id": qid, "createdAt": created_at})
        return saved

    def get_questions(
        self,
        source_file_id: str | None = None,
        difficulty: str | None = None,
        exam_style: str | None = None,
        limit: int | None = None,
    ) -> list[dict[str, Any]]:
        query = """SELECT q.*,
                          (SELECT COUNT(*) FROM question_attempts a WHERE a.question_id = q.id)
                              AS attempt_count
                   FROM questions q WHERE 1=1"""
        params: list[Any] = []
        if source_file_id:
            query += " AND q.source_file_id = ?"
            params.append(source_file_id)
        if difficulty:
            query += " AND q.difficulty = ?"
            params.append(difficulty)
        if exam_style:
            query += " AND q.exam_style = ?"
            params.append(exam_style)
        query += " ORDER BY q.created_at DESC"
        if limit:
            query += " LIMIT ?"
            params.append(limit)
        with self._conn() as conn:
            rows = conn.execute(query, params).fetchall()
        return [self._row_to_question(r) for r in rows]

    def get_questions_by_ids(self, ids: list[str]) -> list[dict[str, Any]]:
        if not ids:
            return []
        placeholders = ",".join("?" * len(ids))
        with self._conn() as conn:
            rows = conn.execute(
                f"""SELECT q.*,
                           (SELECT COUNT(*) FROM question_attempts a WHERE a.question_id = q.id)
                               AS attempt_count
                    FROM questions q WHERE q.id IN ({placeholders})""",
                ids,
            ).fetchall()
        qmap = {r["id"]: self._row_to_question(r) for r in rows}
        return [qmap[i] for i in ids if i in qmap]

    def get_question_count(self, source_file_id: str | None = None) -> int:
        with self._conn() as conn:
            if source_file_id:
                row = conn.execute(
                    "SELECT COUNT(*) as c FROM questions WHERE source_file_id = ?",
                    (source_file_id,),
                ).fetchone()
            else:
                row = conn.execute("SELECT COUNT(*) as c FROM questions").fetchone()
        return row["c"]

    def delete_question(self, question_id: str) -> None:
        with self._conn() as conn:
            conn.execute("DELETE FROM questions WHERE id = ?", (question_id,))

    def record_question_attempt(
        self,
        question_id: str,
        selected_answer: str,
        is_correct: bool,
        quiz_session_id: str | None = None,
    ) -> None:
        attempt_id = str(uuid.uuid4())
        answered_at = datetime.now(timezone.utc).isoformat()
        last_result = "correct" if is_correct else "incorrect"
        with self._conn() as conn:
            conn.execute(
                """INSERT INTO question_attempts
                   (id, question_id, quiz_session_id, selected_answer, is_correct, answered_at)
                   VALUES (?, ?, ?, ?, ?, ?)""",
                (
                    attempt_id,
                    question_id,
                    quiz_session_id,
                    selected_answer,
                    int(is_correct),
                    answered_at,
                ),
            )
            conn.execute(
                """UPDATE questions
                   SET last_result = ?, last_answered_at = ?
                   WHERE id = ?""",
                (last_result, answered_at, question_id),
            )

    def get_question_attempts(self, question_id: str) -> list[dict[str, Any]]:
        with self._conn() as conn:
            rows = conn.execute(
                """SELECT id, question_id, quiz_session_id, selected_answer,
                          is_correct, answered_at
                   FROM question_attempts
                   WHERE question_id = ?
                   ORDER BY answered_at DESC""",
                (question_id,),
            ).fetchall()
        return [
            {
                "id": r["id"],
                "questionId": r["question_id"],
                "quizSessionId": r["quiz_session_id"],
                "selectedAnswer": r["selected_answer"],
                "isCorrect": bool(r["is_correct"]),
                "answeredAt": r["answered_at"],
            }
            for r in rows
        ]

    def create_quiz_session(
        self, source_file_id: str, settings: dict, question_ids: list[str]
    ) -> dict[str, Any]:
        session_id = str(uuid.uuid4())
        started_at = datetime.now(timezone.utc).isoformat()
        with self._conn() as conn:
            conn.execute(
                """INSERT INTO quiz_sessions
                   (id, source_file_id, settings, question_ids, started_at)
                   VALUES (?, ?, ?, ?, ?)""",
                (
                    session_id,
                    source_file_id,
                    json.dumps(settings),
                    json.dumps(question_ids),
                    started_at,
                ),
            )
        return {
            "id": session_id,
            "sourceFileId": source_file_id,
            "settings": settings,
            "questionIds": question_ids,
            "startedAt": started_at,
            "completedAt": None,
            "score": None,
        }

    def complete_quiz_session(self, session_id: str, score: float) -> None:
        with self._conn() as conn:
            conn.execute(
                "UPDATE quiz_sessions SET completed_at = ?, score = ? WHERE id = ?",
                (datetime.now(timezone.utc).isoformat(), score, session_id),
            )

    def get_app_settings(self) -> dict[str, str]:
        """Read settings from DB (legacy camelCase keys), return Python-style keys."""
        db_defaults = {"apiKey": "", "apiBaseUrl": "https://api.openai.com/v1", "model": "gpt-4o-mini"}
        with self._conn() as conn:
            rows = conn.execute("SELECT key, value FROM app_settings").fetchall()
        for row in rows:
            if row["key"] in db_defaults:
                db_defaults[row["key"]] = row["value"]
        return {
            "api_key": db_defaults["apiKey"],
            "api_base_url": db_defaults["apiBaseUrl"],
            "model": db_defaults["model"],
        }

    def save_app_settings(self, settings: dict[str, str]) -> None:
        """Persist settings using legacy DB key names for backward compatibility."""
        mapping = {
            "apiKey": settings.get("api_key", ""),
            "apiBaseUrl": settings.get("api_base_url", ""),
            "model": settings.get("model", ""),
        }
        with self._conn() as conn:
            for key, value in mapping.items():
                conn.execute(
                    "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?, ?)",
                    (key, value),
                )

    def get_preference(self, key: str, default: str = "") -> str:
        with self._conn() as conn:
            row = conn.execute(
                "SELECT value FROM app_settings WHERE key = ?", (key,)
            ).fetchone()
        return row["value"] if row else default

    def set_preference(self, key: str, value: str) -> None:
        with self._conn() as conn:
            conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?, ?)",
                (key, value),
            )

    def _row_to_question(self, row: sqlite3.Row) -> dict[str, Any]:
        keys = row.keys()
        citation = None
        if "citation_quote" in keys and row["citation_quote"]:
            citation = {
                "filename": row["citation_filename"],
                "professorName": row["citation_professor"],
                "quote": row["citation_quote"],
            }
        subjects: list[str] = []
        if "subjects" in keys and row["subjects"]:
            try:
                subjects = json.loads(row["subjects"])
            except json.JSONDecodeError:
                subjects = []

        last_result = row["last_result"] if "last_result" in keys else None
        last_answered_at = row["last_answered_at"] if "last_answered_at" in keys else None
        attempt_count = int(row["attempt_count"]) if "attempt_count" in keys else 0

        return {
            "id": row["id"],
            "sourceFileId": row["source_file_id"],
            "stem": row["stem"],
            "options": json.loads(row["options"]),
            "correctAnswer": row["correct_answer"],
            "explanation": row["explanation"],
            "difficulty": row["difficulty"],
            "examStyle": row["exam_style"],
            "topic": row["topic"],
            "subjects": subjects,
            "lastResult": last_result,
            "lastAnsweredAt": last_answered_at,
            "attemptCount": attempt_count,
            "createdAt": row["created_at"],
            "citation": citation,
        }