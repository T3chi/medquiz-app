import os
from pathlib import Path

APP_NAME = "MedQuiz"

DIFFICULTY_LABELS = {
    "definition": "Definition",
    "first_order": "First Order",
    "second_order": "Second Order",
}

DIFFICULTY_DESCRIPTIONS = {
    "definition": (
        "Foundational knowledge with NBME one-best-answer structure; "
        "concise vignettes or science contexts with closed lead-ins"
    ),
    "first_order": (
        "NBME application-level items: clinical vignette with single-step reasoning "
        "(diagnosis, next step, or mechanism)"
    ),
    "second_order": (
        "NBME application-level items: multi-step integration, differential diagnosis, "
        "or synthesis of multiple findings"
    ),
}

MIN_QUESTION_COUNT = 1
MAX_QUESTION_COUNT = 100

DEFAULT_QUIZ_SETTINGS = {
    "question_count": 10,
    "difficulty": "first_order",
    "answer_timing": "per_question",
    "exam_style": "USMLE",
}

DEFAULT_APP_SETTINGS = {
    "api_key": "",
    "api_base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
}


def get_app_data_dir() -> Path:
    """Standard app data path; existing databases in this folder are preserved across updates."""
    if os.name == "nt":
        base = Path(os.environ.get("APPDATA", Path.home() / "AppData" / "Roaming"))
    elif os.name == "darwin":
        base = Path.home() / "Library" / "Application Support"
    else:
        base = Path.home() / ".config"
    path = base / "medquiz-app"
    path.mkdir(parents=True, exist_ok=True)
    return path


def get_db_path() -> Path:
    return get_app_data_dir() / "medquiz.db"


def parse_question_count(raw: str, fallback: int = DEFAULT_QUIZ_SETTINGS["question_count"]) -> int | None:
    """Return an integer question count in range, or None if invalid."""
    try:
        value = int(str(raw).strip())
    except (TypeError, ValueError):
        return None
    if MIN_QUESTION_COUNT <= value <= MAX_QUESTION_COUNT:
        return value
    return None


def get_uploads_dir() -> Path:
    path = get_app_data_dir() / "uploads"
    path.mkdir(parents=True, exist_ok=True)
    return path