import customtkinter as ctk

from app.config import DIFFICULTY_LABELS
from app.explanations import format_option_explanations
from app.source_metadata import format_citation_display
from app.ui import theme as T


class QuizViewFrame(ctk.CTkFrame):
    def __init__(self, master, db, question_ids, settings, source_file_id, on_exit, **kwargs):
        super().__init__(master, fg_color=T.BG_PRIMARY, **kwargs)
        self.db = db
        self.settings = settings
        self.on_exit = on_exit
        self.questions = db.get_questions_by_ids(question_ids)
        self.session = db.create_quiz_session(source_file_id, settings, question_ids)
        self.index = 0
        self.answers: dict[str, str] = {}
        self.selected: str | None = None
        self.showing_result = False
        self.complete = False
        self.option_buttons: list[ctk.CTkButton] = []

        self._build_ui()
        self._show_question()

    def _build_ui(self):
        top = ctk.CTkFrame(self, fg_color="transparent")
        top.pack(fill="x", padx=32, pady=(20, 8))
        ctk.CTkButton(
            top, text="← Exit Quiz", fg_color="transparent", hover_color=T.BG_ELEVATED,
            text_color=T.TEXT_SECONDARY, command=self.on_exit,
        ).pack(side="left")
        badges = ctk.CTkFrame(top, fg_color="transparent")
        badges.pack(side="right")
        ctk.CTkLabel(
            badges, text=self.settings["exam_style"], font=T.FONT_SMALL,
            fg_color=T.BG_TERTIARY, corner_radius=12, padx=10, pady=4,
        ).pack(side="left", padx=4)
        ctk.CTkLabel(
            badges, text=DIFFICULTY_LABELS[self.settings["difficulty"]], font=T.FONT_SMALL,
            fg_color=T.BG_TERTIARY, corner_radius=12, padx=10, pady=4,
        ).pack(side="left")

        self.progress_label = ctk.CTkLabel(
            self, text="", font=T.FONT_BODY, text_color=T.TEXT_SECONDARY
        )
        self.progress_label.pack(anchor="w", padx=32, pady=(0, 8))

        self.card = ctk.CTkFrame(self, fg_color=T.BG_SECONDARY, corner_radius=12, border_width=1, border_color=T.BORDER)
        self.card.pack(fill="both", expand=True, padx=32, pady=8)

        self.topic_label = ctk.CTkLabel(self.card, text="", font=T.FONT_SMALL, text_color=T.ACCENT)
        self.topic_label.pack(anchor="w", padx=24, pady=(16, 0))

        self.stem_label = ctk.CTkLabel(
            self.card, text="", font=T.FONT_BODY, text_color=T.TEXT_PRIMARY,
            wraplength=700, justify="left", anchor="w",
        )
        self.stem_label.pack(fill="x", padx=24, pady=16)

        self.options_frame = ctk.CTkFrame(self.card, fg_color="transparent")
        self.options_frame.pack(fill="x", padx=20, pady=(0, 8))

        self.explanation_frame = ctk.CTkFrame(self.card, fg_color=T.BG_TERTIARY, corner_radius=8)
        self.explanation_label = ctk.CTkLabel(
            self.explanation_frame, text="", font=T.FONT_BODY, text_color=T.TEXT_SECONDARY,
            wraplength=680, justify="left", anchor="w",
        )
        self.explanation_label.pack(padx=16, pady=12)

        action = ctk.CTkFrame(self, fg_color="transparent")
        action.pack(fill="x", padx=32, pady=16)
        self.action_btn = ctk.CTkButton(
            action, text="Submit Answer", font=T.FONT_HEADING, height=40,
            fg_color=T.ACCENT, hover_color=T.ACCENT_HOVER, command=self._action,
        )
        self.action_btn.pack(side="right")

        self.results_frame = ctk.CTkScrollableFrame(self, fg_color=T.BG_PRIMARY)

    def _show_question(self):
        if self.complete:
            return
        q = self.questions[self.index]
        total = len(self.questions)
        self.progress_label.configure(text=f"Question {self.index + 1} of {total}")
        self.topic_label.configure(text=self._format_tags(q))
        self.stem_label.configure(text=q["stem"])
        self.explanation_frame.pack_forget()
        self.showing_result = False
        self.selected = self.answers.get(q["id"])

        for btn in self.option_buttons:
            btn.destroy()
        self.option_buttons.clear()

        for opt in q["options"]:
            label = opt["label"]
            btn = ctk.CTkButton(
                self.options_frame,
                text=f"  {label}.  {opt['text']}",
                font=T.FONT_BODY,
                anchor="w",
                height=48,
                fg_color=T.ACCENT if self.selected == label else T.BG_TERTIARY,
                hover_color=T.BG_ELEVATED,
                border_width=1,
                border_color=T.ACCENT if self.selected == label else T.BORDER,
                command=lambda l=label: self._select(l),
            )
            btn.pack(fill="x", pady=4)
            self.option_buttons.append(btn)

        per_q = self.settings["answer_timing"] == "per_question"
        if self.index == total - 1 and not per_q:
            self.action_btn.configure(text="Finish Quiz")
        elif per_q:
            self.action_btn.configure(text="Submit Answer")
        else:
            self.action_btn.configure(text="Next Question")

    def _select(self, label: str):
        if self.showing_result and self.settings["answer_timing"] == "per_question":
            return
        self.selected = label
        q = self.questions[self.index]
        for opt in q["options"]:
            for btn in self.option_buttons:
                if btn.cget("text").startswith(f"  {opt['label']}."):
                    btn.configure(
                        fg_color=T.ACCENT if opt["label"] == label else T.BG_TERTIARY,
                        border_color=T.ACCENT if opt["label"] == label else T.BORDER,
                    )

    def _action(self):
        q = self.questions[self.index]
        per_q = self.settings["answer_timing"] == "per_question"

        if not self.showing_result:
            if not self.selected:
                return
            self.answers[q["id"]] = self.selected
            self._record_answer(q, self.selected)
            if per_q:
                self._show_explanation(q)
                self.showing_result = True
                last = self.index == len(self.questions) - 1
                self.action_btn.configure(text="View Results" if last else "Next Question")
            elif self.index == len(self.questions) - 1:
                self._show_results()
            else:
                self.index += 1
                self.selected = None
                self._show_question()
        else:
            if self.index == len(self.questions) - 1:
                self._show_results()
            else:
                self.index += 1
                self.selected = None
                self._show_question()

    def _record_answer(self, q: dict, selected: str) -> None:
        self.db.record_question_attempt(
            question_id=q["id"],
            selected_answer=selected,
            is_correct=selected == q["correctAnswer"],
            quiz_session_id=self.session["id"],
        )

    def _format_tags(self, q: dict) -> str:
        subjects = q.get("subjects") or []
        if subjects:
            return " · ".join(subjects)
        return q.get("topic") or ""

    def _show_explanation(self, q: dict):
        correct = q["correctAnswer"]
        ok = self.selected == correct
        header = "Correct!" if ok else f"Incorrect — Answer: {correct}"
        color = T.SUCCESS if ok else T.ERROR
        citation_text = format_citation_display(q.get("citation"))
        body = format_option_explanations(q)
        if citation_text:
            body = f"{body}\n\n{citation_text}"
        self.explanation_label.configure(text=f"{header}\n\n{body}", text_color=color)
        self.explanation_frame.pack(fill="x", padx=20, pady=(8, 16))
        for opt in q["options"]:
            for btn in self.option_buttons:
                if btn.cget("text").startswith(f"  {opt['label']}."):
                    if opt["label"] == correct:
                        btn.configure(fg_color=T.SUCCESS, border_color=T.SUCCESS)
                    elif opt["label"] == self.selected and not ok:
                        btn.configure(fg_color=T.ERROR, border_color=T.ERROR)

    def _show_results(self):
        self.complete = True
        self.card.pack_forget()
        self.action_btn.pack_forget()
        self.progress_label.configure(text="Quiz Complete")

        correct_count = sum(
            1 for q in self.questions if self.answers.get(q["id"]) == q["correctAnswer"]
        )
        total = len(self.questions)
        score = (correct_count / total * 100) if total else 0
        self.db.complete_quiz_session(self.session["id"], score)

        self.results_frame.pack(fill="both", expand=True, padx=32, pady=8)
        ctk.CTkLabel(
            self.results_frame,
            text=f"{score:.0f}%  ({correct_count}/{total} correct)",
            font=T.FONT_TITLE,
            text_color=T.SUCCESS if score >= 70 else T.ERROR,
        ).pack(pady=16)

        for i, q in enumerate(self.questions):
            ok = self.answers.get(q["id"]) == q["correctAnswer"]
            card = ctk.CTkFrame(
                self.results_frame, fg_color=T.BG_SECONDARY, corner_radius=10,
                border_width=1, border_color=T.SUCCESS if ok else T.ERROR,
            )
            card.pack(fill="x", pady=6)
            status = "Correct" if ok else "Incorrect"
            tags = self._format_tags(q)
            title = f"Q{i+1} — {status}"
            if tags:
                title = f"{title} · {tags}"
            ctk.CTkLabel(
                card, text=title, font=T.FONT_SMALL,
                text_color=T.SUCCESS if ok else T.ERROR,
            ).pack(anchor="w", padx=16, pady=(10, 0))
            ctk.CTkLabel(
                card, text=q["stem"], font=T.FONT_BODY, wraplength=700,
                justify="left", anchor="w", text_color=T.TEXT_PRIMARY,
            ).pack(anchor="w", padx=16, pady=6)
            ctk.CTkLabel(
                card,
                text=f"Your answer: {self.answers.get(q['id'], '—')}  |  Correct: {q['correctAnswer']}",
                font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY,
            ).pack(anchor="w", padx=16)
            result_body = format_option_explanations(q)
            citation_text = format_citation_display(q.get("citation"))
            if citation_text:
                result_body = f"{result_body}\n\n{citation_text}"
            ctk.CTkLabel(
                card, text=result_body, font=T.FONT_SMALL, wraplength=700,
                justify="left", anchor="w", text_color=T.TEXT_MUTED,
            ).pack(anchor="w", padx=16, pady=(4, 12))

        ctk.CTkButton(
            self, text="Back to Home", font=T.FONT_HEADING, height=40,
            fg_color=T.ACCENT, hover_color=T.ACCENT_HOVER, command=self.on_exit,
        ).pack(pady=16)