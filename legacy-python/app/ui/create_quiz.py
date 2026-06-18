import shutil
import threading
import time
from pathlib import Path
from tkinter import filedialog, messagebox

import customtkinter as ctk

from app.config import (
    DEFAULT_QUIZ_SETTINGS,
    DIFFICULTY_DESCRIPTIONS,
    DIFFICULTY_LABELS,
    MAX_QUESTION_COUNT,
    MIN_QUESTION_COUNT,
    get_uploads_dir,
    parse_question_count,
)
from app.file_parser import parse_file
from app.quiz_service import QuizService
from app.ui import theme as T

LAST_UPLOAD_DIR_KEY = "last_upload_dir"


class CreateQuizFrame(ctk.CTkFrame):
    def __init__(self, master, db, on_quiz_ready, **kwargs):
        super().__init__(master, fg_color=T.BG_PRIMARY, **kwargs)
        self.db = db
        self.on_quiz_ready = on_quiz_ready
        self.files: list[dict] = []
        self.selected_file_id: str | None = None
        self.settings = dict(DEFAULT_QUIZ_SETTINGS)
        self._building = False

        self._build_header()
        self._build_body()
        self.refresh_files()

    def _build_header(self):
        header = ctk.CTkFrame(self, fg_color="transparent")
        header.pack(fill="x", padx=32, pady=(28, 16))
        ctk.CTkLabel(
            header, text="Create Quiz", font=T.FONT_TITLE, text_color=T.TEXT_PRIMARY
        ).pack(anchor="w")
        ctk.CTkLabel(
            header,
            text="Upload lecture slides or notes and generate board-style practice questions.",
            font=T.FONT_BODY,
            text_color=T.TEXT_SECONDARY,
        ).pack(anchor="w", pady=(4, 0))

    def _build_body(self):
        body = ctk.CTkFrame(self, fg_color="transparent")
        body.pack(fill="both", expand=True, padx=32, pady=(0, 24))
        body.grid_columnconfigure(0, weight=1)
        body.grid_columnconfigure(1, weight=1)
        body.grid_rowconfigure(0, weight=1)

        self._build_files_panel(body)
        self._build_settings_panel(body)

    def _card(self, parent, title: str) -> ctk.CTkFrame:
        card = ctk.CTkFrame(parent, fg_color=T.BG_SECONDARY, corner_radius=12, border_width=1, border_color=T.BORDER)
        card.grid(sticky="nsew", padx=(0, 8) if title == "Study Materials" else (8, 0))
        ctk.CTkLabel(card, text=title, font=T.FONT_HEADING, text_color=T.TEXT_PRIMARY).pack(
            anchor="w", padx=20, pady=(16, 12)
        )
        return card

    def _build_files_panel(self, parent):
        card = self._card(parent, "Study Materials")
        card.grid(row=0, column=0, sticky="nsew")

        upload_btn = ctk.CTkButton(
            card,
            text="Upload PDF or PowerPoint",
            font=T.FONT_BODY,
            fg_color=T.BG_TERTIARY,
            hover_color=T.BG_ELEVATED,
            border_width=1,
            border_color=T.BORDER,
            height=80,
            command=self._upload_file,
        )
        upload_btn.pack(fill="x", padx=20, pady=(0, 12))

        self.file_list = ctk.CTkScrollableFrame(card, fg_color="transparent", height=320)
        self.file_list.pack(fill="both", expand=True, padx=16, pady=(0, 16))

    def _build_settings_panel(self, parent):
        card = self._card(parent, "Quiz Settings")
        card.grid(row=0, column=1, sticky="nsew")

        inner = ctk.CTkFrame(card, fg_color="transparent")
        inner.pack(fill="both", expand=True, padx=20, pady=(0, 16))

        ctk.CTkLabel(inner, text="Number of Questions", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        count_row = ctk.CTkFrame(inner, fg_color="transparent")
        count_row.pack(fill="x", pady=(4, 16))
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

        ctk.CTkLabel(inner, text="Exam Style", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        self.exam_var = ctk.StringVar(value=self.settings["exam_style"])
        exam_row = ctk.CTkFrame(inner, fg_color="transparent")
        exam_row.pack(fill="x", pady=(4, 16))
        for style in ("USMLE", "COMLEX"):
            ctk.CTkRadioButton(
                exam_row, text=style, variable=self.exam_var, value=style,
                font=T.FONT_BODY, fg_color=T.ACCENT, hover_color=T.ACCENT_HOVER,
                command=lambda: self._set("exam_style", self.exam_var.get()),
            ).pack(side="left", padx=(0, 16))

        ctk.CTkLabel(inner, text="Difficulty", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        self.diff_var = ctk.StringVar(value=self.settings["difficulty"])
        for key, label in DIFFICULTY_LABELS.items():
            ctk.CTkRadioButton(
                inner,
                text=f"{label} — {DIFFICULTY_DESCRIPTIONS[key]}",
                variable=self.diff_var,
                value=key,
                font=T.FONT_SMALL,
                fg_color=T.ACCENT,
                hover_color=T.ACCENT_HOVER,
                command=lambda: self._set("difficulty", self.diff_var.get()),
            ).pack(anchor="w", pady=2)

        ctk.CTkLabel(inner, text="Show Answers", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w", pady=(12, 0))
        self.timing_var = ctk.StringVar(value=self.settings["answer_timing"])
        timing_row = ctk.CTkFrame(inner, fg_color="transparent")
        timing_row.pack(fill="x", pady=(4, 16))
        ctk.CTkRadioButton(
            timing_row, text="After Each Question", variable=self.timing_var, value="per_question",
            font=T.FONT_BODY, fg_color=T.ACCENT, command=lambda: self._set("answer_timing", self.timing_var.get()),
        ).pack(side="left", padx=(0, 12))
        ctk.CTkRadioButton(
            timing_row, text="End of Quiz", variable=self.timing_var, value="end_of_quiz",
            font=T.FONT_BODY, fg_color=T.ACCENT, command=lambda: self._set("answer_timing", self.timing_var.get()),
        ).pack(side="left")

        self.progress_frame = ctk.CTkFrame(inner, fg_color="transparent")

        progress_header = ctk.CTkFrame(self.progress_frame, fg_color="transparent")
        progress_header.pack(fill="x")
        self.progress_label = ctk.CTkLabel(
            progress_header, text="", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY, anchor="w",
        )
        self.progress_label.pack(side="left", fill="x", expand=True)
        self.progress_pct_label = ctk.CTkLabel(
            progress_header, text="", font=T.FONT_SMALL, text_color=T.ACCENT, anchor="e", width=48,
        )
        self.progress_pct_label.pack(side="right")

        self.progress_bar = ctk.CTkProgressBar(
            self.progress_frame, progress_color=T.ACCENT, fg_color=T.BG_TERTIARY, height=10,
        )
        self.progress_bar.pack(fill="x", pady=(6, 0))
        self.progress_bar.set(0)

        self.generate_btn = ctk.CTkButton(
            inner, text="Generate Quiz", font=T.FONT_HEADING, height=44,
            fg_color=T.ACCENT, hover_color=T.ACCENT_HOVER, command=self._generate,
        )
        self.generate_btn.pack(fill="x", pady=(12, 0))

    def _set(self, key: str, value):
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
        return True

    def refresh_files(self):
        self.files = self.db.get_source_files()
        if self.files and not self.selected_file_id:
            self.selected_file_id = self.files[0]["id"]
        for w in self.file_list.winfo_children():
            w.destroy()
        for f in self.files:
            self._add_file_row(f)

    def _add_file_row(self, f: dict):
        selected = f["id"] == self.selected_file_id
        row = ctk.CTkFrame(
            self.file_list,
            fg_color=T.ACCENT if selected else T.BG_TERTIARY,
            corner_radius=8,
            border_width=1,
            border_color=T.ACCENT if selected else T.BORDER,
        )
        row.pack(fill="x", pady=4)
        icon = "PDF" if f["fileType"] == "pdf" else "PPT"
        meta = f"{f['textLength'] // 1000}k chars"
        ctk.CTkLabel(row, text=f"[{icon}] {f['filename']}", font=T.FONT_BODY, anchor="w").pack(
            side="left", padx=12, pady=10
        )
        ctk.CTkLabel(row, text=meta, font=T.FONT_SMALL, text_color=T.TEXT_MUTED).pack(side="left")
        ctk.CTkButton(
            row, text="X", width=28, height=28, fg_color="transparent", hover_color=T.ERROR,
            command=lambda fid=f["id"]: self._delete_file(fid),
        ).pack(side="right", padx=8)
        row.bind("<Button-1>", lambda _e, fid=f["id"]: self._select_file(fid))
        for child in row.winfo_children():
            if not isinstance(child, ctk.CTkButton):
                child.bind("<Button-1>", lambda _e, fid=f["id"]: self._select_file(fid))

    def _select_file(self, file_id: str):
        self.selected_file_id = file_id
        self.refresh_files()

    def _delete_file(self, file_id: str):
        self.db.delete_source_file(file_id)
        if self.selected_file_id == file_id:
            self.selected_file_id = None
        self.refresh_files()

    def _upload_file(self):
        last_dir = self.db.get_preference(LAST_UPLOAD_DIR_KEY)
        kwargs: dict = {
            "title": "Select PDF or PowerPoint",
            "filetypes": [
                ("Study Materials", "*.pdf *.pptx *.ppt"),
                ("PDF", "*.pdf"),
                ("PowerPoint", "*.pptx *.ppt"),
            ],
        }
        if last_dir and Path(last_dir).is_dir():
            kwargs["initialdir"] = last_dir

        path = filedialog.askopenfilename(**kwargs)
        if not path:
            return
        src = Path(path)
        self.db.set_preference(LAST_UPLOAD_DIR_KEY, str(src.parent))
        try:
            text, file_type = parse_file(path)
            dest = get_uploads_dir() / f"{int(time.time())}_{src.name}"
            shutil.copy2(path, dest)
            record = self.db.save_source_file(src.name, str(dest), file_type, text)
            self.selected_file_id = record["id"]
            self.refresh_files()
        except Exception as exc:
            messagebox.showerror("Upload Failed", str(exc))

    def _generate(self):
        if not self.selected_file_id:
            messagebox.showwarning("No File", "Please select or upload a study file first.")
            return
        if not self._apply_question_count(show_error=True):
            return
        if self._building:
            return
        self._building = True
        self.generate_btn.configure(state="disabled", text="Generating...")
        self.progress_frame.pack(fill="x", pady=(12, 0), before=self.generate_btn)
        self._set_progress("Starting...", 0)
        threading.Thread(target=self._generate_worker, daemon=True).start()

    def _generate_worker(self):
        try:
            app_settings = self.db.get_app_settings()
            service = QuizService(self.db)
            ids = service.generate_quiz(
                source_file_id=self.selected_file_id,
                settings=dict(self.settings),
                app_settings=app_settings,
                on_progress=lambda msg, pct: self._set_progress(msg, pct),
            )
            self.after(0, lambda: self.on_quiz_ready(ids, dict(self.settings), self.selected_file_id))
        except Exception as exc:
            self.after(0, lambda: messagebox.showerror("Generation Failed", str(exc)))
        finally:
            self.after(0, self._reset_generate)

    def _set_progress(self, msg: str, percent: float = 0):
        def update():
            pct = min(100.0, max(0.0, percent))
            self.progress_label.configure(text=msg)
            self.progress_pct_label.configure(text=f"{int(pct)}%")
            self.progress_bar.set(pct / 100.0)
            self.update_idletasks()

        self.after(0, update)

    def _reset_generate(self):
        self._building = False
        self.generate_btn.configure(state="normal", text="Generate Quiz")
        self.progress_frame.pack_forget()
        self.progress_label.configure(text="")
        self.progress_pct_label.configure(text="")
        self.progress_bar.set(0)