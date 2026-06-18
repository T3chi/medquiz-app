import customtkinter as ctk

from app.config import APP_NAME
from app.database import Database
from app.ui import theme as T
from app.ui.create_quiz import CreateQuizFrame
from app.ui.question_bank import QuestionBankFrame
from app.ui.quiz_view import QuizViewFrame
from app.ui.settings import SettingsFrame
from app.ui.take_quiz import TakeQuizFrame


class MainWindow(ctk.CTk):
    def __init__(self):
        super().__init__()
        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")

        self.title(f"{APP_NAME} — USMLE & COMLEX Practice")
        self.geometry("1100x720")
        self.minsize(900, 600)
        self.configure(fg_color=T.BG_PRIMARY)

        self.db = Database()
        self.current_view: str = "create"
        self.frames: dict = {}
        self.quiz_frame: QuizViewFrame | None = None

        self.grid_columnconfigure(1, weight=1)
        self.grid_rowconfigure(0, weight=1)

        self._build_sidebar()
        self._build_content()
        self.show_view("create")

    def _build_sidebar(self):
        sidebar = ctk.CTkFrame(self, width=T.SIDEBAR_WIDTH, fg_color=T.BG_SECONDARY, corner_radius=0)
        sidebar.grid(row=0, column=0, sticky="nsew")
        sidebar.grid_propagate(False)

        logo = ctk.CTkFrame(sidebar, fg_color="transparent")
        logo.pack(fill="x", padx=16, pady=(24, 20))
        ctk.CTkLabel(logo, text="⚕ MedQuiz", font=T.FONT_HEADING, text_color=T.TEXT_PRIMARY).pack(anchor="w")
        ctk.CTkLabel(logo, text="USMLE & COMLEX Prep", font=T.FONT_SMALL, text_color=T.TEXT_MUTED).pack(anchor="w")

        nav_items = [
            ("create", "✦  Create Quiz"),
            ("quiz", "▶  Take Quiz"),
            ("bank", "◫  Question Bank"),
            ("settings", "⚙  Settings"),
        ]
        self.nav_buttons: dict[str, ctk.CTkButton] = {}
        for key, label in nav_items:
            btn = ctk.CTkButton(
                sidebar, text=label, anchor="w", font=T.FONT_BODY, height=40,
                fg_color="transparent", hover_color=T.BG_ELEVATED,
                text_color=T.TEXT_SECONDARY,
                command=lambda k=key: self.show_view(k),
            )
            btn.pack(fill="x", padx=12, pady=3)
            self.nav_buttons[key] = btn

    def _build_content(self):
        self.content = ctk.CTkFrame(self, fg_color=T.BG_PRIMARY, corner_radius=0)
        self.content.grid(row=0, column=1, sticky="nsew")
        self.content.grid_rowconfigure(0, weight=1)
        self.content.grid_columnconfigure(0, weight=1)

        self.frames["create"] = CreateQuizFrame(
            self.content, self.db, on_quiz_ready=self.start_quiz
        )
        self.frames["quiz"] = TakeQuizFrame(self.content, self.db, on_start=self.start_quiz)
        self.frames["bank"] = QuestionBankFrame(self.content, self.db)
        self.frames["settings"] = SettingsFrame(self.content, self.db)

    def show_view(self, name: str):
        if self.quiz_frame:
            return
        self.current_view = name
        for key, frame in self.frames.items():
            if key == name:
                frame.grid(row=0, column=0, sticky="nsew")
            else:
                frame.grid_forget()
        for key, btn in self.nav_buttons.items():
            if key == name:
                btn.configure(fg_color=T.BG_ELEVATED, text_color=T.ACCENT)
            else:
                btn.configure(fg_color="transparent", text_color=T.TEXT_SECONDARY)

    def start_quiz(self, question_ids, settings, source_file_id):
        for frame in self.frames.values():
            frame.grid_forget()
        self.quiz_frame = QuizViewFrame(
            self.content, self.db, question_ids, settings, source_file_id,
            on_exit=self.exit_quiz,
        )
        self.quiz_frame.grid(row=0, column=0, sticky="nsew")

    def exit_quiz(self):
        if self.quiz_frame:
            self.quiz_frame.destroy()
            self.quiz_frame = None
        bank = self.frames.get("bank")
        if bank:
            bank.refresh()
        create = self.frames.get("create")
        if create:
            create.refresh_files()
        self.show_view("create")