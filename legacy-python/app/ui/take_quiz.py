import random
from tkinter import messagebox

import customtkinter as ctk

from app.config import (
    DEFAULT_QUIZ_SETTINGS,
    DIFFICULTY_LABELS,
    MAX_QUESTION_COUNT,
    MIN_QUESTION_COUNT,
    parse_question_count,
)
from app.ui import theme as T


class TakeQuizFrame(ctk.CTkFrame):
    def __init__(self, master, db, on_start, **kwargs):
        super().__init__(master, fg_color=T.BG_PRIMARY, **kwargs)
        self.db = db
        self.on_start = on_start
        self.settings = dict(DEFAULT_QUIZ_SETTINGS)
        self.filter_file_id = ""
        self._build()

    def _build(self):
        ctk.CTkLabel(self, text="Take Quiz", font=T.FONT_TITLE, text_color=T.TEXT_PRIMARY).pack(
            anchor="w", padx=32, pady=(28, 4)
        )
        ctk.CTkLabel(
            self, text="Practice with questions from your bank using custom settings.",
            font=T.FONT_BODY, text_color=T.TEXT_SECONDARY,
        ).pack(anchor="w", padx=32, pady=(0, 20))

        self.all_questions = self.db.get_questions()
        if not self.all_questions:
            ctk.CTkLabel(
                self, text="No questions available.\nCreate a quiz first to generate questions.",
                font=T.FONT_BODY, text_color=T.TEXT_MUTED,
            ).pack(pady=60)
            return

        card = ctk.CTkFrame(self, fg_color=T.BG_SECONDARY, corner_radius=12, border_width=1, border_color=T.BORDER)
        card.pack(fill="x", padx=32, pady=8)

        inner = ctk.CTkFrame(card, fg_color="transparent")
        inner.pack(fill="x", padx=24, pady=20)

        ctk.CTkLabel(inner, text="Number of Questions", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        count_row = ctk.CTkFrame(inner, fg_color="transparent")
        count_row.pack(fill="x", pady=(4, 4))
        self.count_var = ctk.StringVar(value=str(self.settings["question_count"]))
        self.count_entry = ctk.CTkEntry(
            count_row,
            textvariable=self.count_var,
            width=72,
            font=T.FONT_BODY,
            fg_color=T.BG_TERTIARY,
            border_color=T.BORDER,
            justify="center",
        )
        self.count_entry.pack(side="left")
        self.count_entry.bind("<FocusOut>", lambda _e: self._apply_question_count())
        self.count_entry.bind("<Return>", lambda _e: self._apply_question_count())
        ctk.CTkLabel(
            count_row,
            text=f"questions ({MIN_QUESTION_COUNT}–{MAX_QUESTION_COUNT})",
            font=T.FONT_SMALL,
            text_color=T.TEXT_MUTED,
        ).pack(side="left", padx=(8, 0))
        self.count_label = ctk.CTkLabel(inner, text="", font=T.FONT_BODY, text_color=T.ACCENT)
        self.count_label.pack(anchor="w", pady=(0, 12))

        files = self.db.get_source_files()
        ctk.CTkLabel(inner, text="Filter by Source", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        self.file_menu = ctk.CTkOptionMenu(
            inner, values=["All sources"] + [f["filename"] for f in files],
            command=self._on_file_filter, fg_color=T.BG_TERTIARY,
        )
        self.file_menu.pack(fill="x", pady=(4, 12))
        self._file_map = {f["filename"]: f["id"] for f in files}

        row = ctk.CTkFrame(inner, fg_color="transparent")
        row.pack(fill="x", pady=4)
        ctk.CTkLabel(row, text="Exam Style", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        self.exam_menu = ctk.CTkOptionMenu(
            row, values=["USMLE", "COMLEX"], command=lambda v: self._set("exam_style", v),
            fg_color=T.BG_TERTIARY, width=140,
        )
        self.exam_menu.pack(side="left", padx=(0, 16))

        ctk.CTkLabel(row, text="Difficulty", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(side="left")
        self.diff_menu = ctk.CTkOptionMenu(
            row,
            values=[DIFFICULTY_LABELS[k] for k in DIFFICULTY_LABELS],
            command=self._on_difficulty,
            fg_color=T.BG_TERTIARY, width=160,
        )
        self.diff_menu.pack(side="left", padx=(8, 0))
        self._diff_keys = list(DIFFICULTY_LABELS.keys())

        ctk.CTkLabel(inner, text="Show Answers", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w", pady=(12, 0))
        self.timing_var = ctk.StringVar(value="per_question")
        trow = ctk.CTkFrame(inner, fg_color="transparent")
        trow.pack(fill="x", pady=4)
        ctk.CTkRadioButton(
            trow, text="After Each Question", variable=self.timing_var, value="per_question",
            fg_color=T.ACCENT, command=lambda: self._set("answer_timing", "per_question"),
        ).pack(side="left", padx=(0, 12))
        ctk.CTkRadioButton(
            trow, text="End of Quiz", variable=self.timing_var, value="end_of_quiz",
            fg_color=T.ACCENT, command=lambda: self._set("answer_timing", "end_of_quiz"),
        ).pack(side="left")

        self._update_count_label()
        ctk.CTkButton(
            inner, text="Start Quiz", font=T.FONT_HEADING, height=44,
            fg_color=T.ACCENT, hover_color=T.ACCENT_HOVER, command=self._start,
        ).pack(fill="x", pady=(16, 0))

    def _set(self, key, value):
        self.settings[key] = value

    def _apply_question_count(self, show_error: bool = False) -> bool:
        count = parse_question_count(self.count_var.get(), self.settings["question_count"])
        if count is None:
            if show_error:
                messagebox.showwarning(
                    "Invalid Count",
                    f"Enter a whole number between {MIN_QUESTION_COUNT} and {MAX_QUESTION_COUNT}.",
                )
            self.count_var.set(str(self.settings["question_count"]))
            return False
        self.settings["question_count"] = count
        self.count_var.set(str(count))
        self._update_count_label()
        return True

    def _on_file_filter(self, choice: str):
        self.filter_file_id = "" if choice == "All sources" else self._file_map.get(choice, "")
        self._update_count_label()

    def _on_difficulty(self, label: str):
        for key, val in DIFFICULTY_LABELS.items():
            if val == label:
                self.settings["difficulty"] = key
                break
        self._update_count_label()

    def _filtered(self) -> list[dict]:
        result = []
        for q in self.all_questions:
            if self.filter_file_id and q["sourceFileId"] != self.filter_file_id:
                continue
            if q["difficulty"] != self.settings["difficulty"]:
                continue
            if q["examStyle"] != self.settings["exam_style"]:
                continue
            result.append(q)
        return result

    def _update_count_label(self):
        available = len(self._filtered())
        count = min(self.settings["question_count"], available)
        self.count_label.configure(text=f"{count} of {available} matching questions available")

    def _start(self):
        if not self._apply_question_count(show_error=True):
            return
        filtered = self._filtered()
        if not filtered:
            return
        shuffled = filtered.copy()
        random.shuffle(shuffled)
        count = min(self.settings["question_count"], len(shuffled))
        selected = shuffled[:count]
        ids = [q["id"] for q in selected]
        self.on_start(ids, dict(self.settings), selected[0]["sourceFileId"])