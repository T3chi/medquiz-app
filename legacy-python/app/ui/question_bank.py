import customtkinter as ctk

from app.config import DIFFICULTY_LABELS
from app.explanations import format_option_explanations
from app.source_metadata import format_citation_display
from app.ui import theme as T


LAST_RESULT_LABELS = {
    "correct": ("Last: Correct", T.SUCCESS),
    "incorrect": ("Last: Incorrect", T.ERROR),
    None: ("Unanswered", T.TEXT_MUTED),
}


class QuestionBankFrame(ctk.CTkFrame):
    def __init__(self, master, db, **kwargs):
        super().__init__(master, fg_color=T.BG_PRIMARY, **kwargs)
        self.db = db
        self.expanded_id: str | None = None
        self.search_query = ""
        self._build()
        self.refresh()

    def _build(self):
        header = ctk.CTkFrame(self, fg_color="transparent")
        header.pack(fill="x", padx=32, pady=(28, 8))
        ctk.CTkLabel(header, text="Question Bank", font=T.FONT_TITLE, text_color=T.TEXT_PRIMARY).pack(anchor="w")
        self.count_header = ctk.CTkLabel(header, text="", font=T.FONT_BODY, text_color=T.TEXT_SECONDARY)
        self.count_header.pack(anchor="w", pady=(4, 0))

        search_row = ctk.CTkFrame(self, fg_color="transparent")
        search_row.pack(fill="x", padx=32, pady=(8, 0))
        self.search_var = ctk.StringVar()
        self.search_entry = ctk.CTkEntry(
            search_row,
            textvariable=self.search_var,
            placeholder_text="Search by subject tag (e.g. antibiotics, anatomy, pain)",
            font=T.FONT_BODY,
            fg_color=T.BG_SECONDARY,
            border_color=T.BORDER,
        )
        self.search_entry.pack(side="left", fill="x", expand=True)
        self.search_entry.bind("<KeyRelease>", lambda _e: self._on_search_change())
        ctk.CTkButton(
            search_row,
            text="Clear",
            width=64,
            fg_color=T.BG_TERTIARY,
            hover_color=T.BG_ELEVATED,
            command=self._clear_search,
        ).pack(side="right", padx=(8, 0))

        self.tags_frame = ctk.CTkFrame(self, fg_color="transparent")
        self.tags_frame.pack(fill="x", padx=32, pady=(8, 0))

        self.list_frame = ctk.CTkScrollableFrame(self, fg_color="transparent")
        self.list_frame.pack(fill="both", expand=True, padx=32, pady=16)

    def _on_search_change(self):
        self.search_query = self.search_var.get().strip().lower()
        self.refresh()

    def _clear_search(self):
        self.search_var.set("")
        self.search_query = ""
        self.refresh()

    def _apply_tag_filter(self, tag: str):
        self.search_var.set(tag)
        self.search_query = tag.lower()
        self.refresh()

    def _collect_tags(self, questions: list[dict]) -> list[str]:
        seen: set[str] = set()
        tags: list[str] = []
        for q in questions:
            for tag in q.get("subjects") or []:
                if tag not in seen:
                    seen.add(tag)
                    tags.append(tag)
        return sorted(tags)

    def _matches_search(self, q: dict) -> bool:
        if not self.search_query:
            return True
        query = self.search_query
        for tag in q.get("subjects") or []:
            if query in tag.lower():
                return True
        topic = (q.get("topic") or "").lower()
        return query in topic

    def _rebuild_tag_chips(self, all_questions: list[dict]) -> None:
        for w in self.tags_frame.winfo_children():
            w.destroy()
        tags = self._collect_tags(all_questions)
        if not tags:
            return
        ctk.CTkLabel(
            self.tags_frame, text="Tags:", font=T.FONT_SMALL, text_color=T.TEXT_MUTED,
        ).pack(side="left", padx=(0, 8))
        for tag in tags[:20]:
            active = self.search_query == tag.lower()
            ctk.CTkButton(
                self.tags_frame,
                text=tag,
                height=24,
                font=T.FONT_SMALL,
                fg_color=T.ACCENT if active else T.BG_TERTIARY,
                hover_color=T.ACCENT_HOVER if active else T.BG_ELEVATED,
                command=lambda t=tag: self._apply_tag_filter(t),
            ).pack(side="left", padx=(0, 6))

    def refresh(self):
        all_questions = self.db.get_questions()
        questions = [q for q in all_questions if self._matches_search(q)]
        self._rebuild_tag_chips(all_questions)

        if self.search_query:
            self.count_header.configure(
                text=(
                    f"{len(questions)} of {len(all_questions)} question"
                    f"{'s' if len(all_questions) != 1 else ''} matching "
                    f'"{self.search_var.get().strip()}"'
                )
            )
        else:
            self.count_header.configure(
                text=f"{len(all_questions)} question{'s' if len(all_questions) != 1 else ''} stored locally"
            )

        for w in self.list_frame.winfo_children():
            w.destroy()
        if not all_questions:
            ctk.CTkLabel(
                self.list_frame,
                text="No questions yet.\nGenerate a quiz from your study materials to build your bank.",
                font=T.FONT_BODY, text_color=T.TEXT_MUTED,
            ).pack(pady=40)
            return
        if not questions:
            ctk.CTkLabel(
                self.list_frame,
                text=f'No questions match "{self.search_var.get().strip()}".\nTry another subject tag.',
                font=T.FONT_BODY, text_color=T.TEXT_MUTED,
            ).pack(pady=40)
            return
        for i, q in enumerate(questions):
            self._add_row(q, len(questions) - i)

    def _add_row(self, q: dict, number: int):
        expanded = q["id"] == self.expanded_id
        card = ctk.CTkFrame(
            self.list_frame, fg_color=T.BG_SECONDARY, corner_radius=10,
            border_width=1, border_color=T.BORDER,
        )
        card.pack(fill="x", pady=5)

        header = ctk.CTkFrame(card, fg_color="transparent")
        header.pack(fill="x", padx=16, pady=12)
        ctk.CTkLabel(header, text=f"#{number}", font=T.FONT_MONO, text_color=T.TEXT_MUTED).pack(side="left")
        ctk.CTkLabel(
            header, text=f" {q['examStyle']} · {DIFFICULTY_LABELS[q['difficulty']]}",
            font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY,
        ).pack(side="left", padx=8)
        status_label, status_color = LAST_RESULT_LABELS.get(
            q.get("lastResult"), LAST_RESULT_LABELS[None]
        )
        attempts = q.get("attemptCount", 0)
        status_text = status_label if attempts == 0 else f"{status_label} ({attempts} tries)"
        ctk.CTkLabel(
            header, text=status_text, font=T.FONT_SMALL, text_color=status_color,
        ).pack(side="left", padx=8)
        ctk.CTkButton(
            header, text="Delete", width=60, height=28, fg_color="transparent",
            hover_color=T.ERROR, text_color=T.TEXT_MUTED,
            command=lambda qid=q["id"]: self._delete(qid),
        ).pack(side="right")

        subjects = q.get("subjects") or []
        if subjects:
            tags_row = ctk.CTkFrame(card, fg_color="transparent")
            tags_row.pack(fill="x", padx=16, pady=(0, 4))
            for tag in subjects[:5]:
                ctk.CTkButton(
                    tags_row,
                    text=tag,
                    height=22,
                    font=T.FONT_SMALL,
                    text_color=T.ACCENT,
                    fg_color=T.BG_TERTIARY,
                    hover_color=T.BG_ELEVATED,
                    command=lambda t=tag: self._apply_tag_filter(t),
                ).pack(side="left", padx=(0, 6))

        stem = q["stem"][:200] + ("..." if len(q["stem"]) > 200 else "")
        stem_btn = ctk.CTkButton(
            card, text=stem, font=T.FONT_BODY, anchor="w", fg_color="transparent",
            hover_color=T.BG_TERTIARY, text_color=T.TEXT_PRIMARY,
            command=lambda qid=q["id"]: self._toggle(qid),
        )
        stem_btn.pack(fill="x", padx=12, pady=(0, 8))

        if expanded:
            detail = ctk.CTkFrame(card, fg_color=T.BG_TERTIARY, corner_radius=8)
            detail.pack(fill="x", padx=16, pady=(0, 12))
            for opt in q["options"]:
                is_correct = opt["label"] == q["correctAnswer"]
                mark = " ✓" if is_correct else ""
                ctk.CTkLabel(
                    detail, text=f"{opt['label']}. {opt['text']}{mark}", font=T.FONT_SMALL,
                    text_color=T.SUCCESS if is_correct else T.TEXT_SECONDARY, anchor="w",
                ).pack(anchor="w", padx=12, pady=(4, 0))
                opt_expl = (opt.get("explanation") or "").strip()
                if opt_expl:
                    ctk.CTkLabel(
                        detail, text=opt_expl, font=T.FONT_SMALL,
                        text_color=T.SUCCESS if is_correct else T.TEXT_MUTED,
                        wraplength=680, justify="left", anchor="w",
                    ).pack(anchor="w", padx=24, pady=(0, 4))
            if subjects:
                ctk.CTkLabel(
                    detail, text=f"Subjects: {', '.join(subjects)}", font=T.FONT_SMALL,
                    text_color=T.ACCENT, anchor="w",
                ).pack(anchor="w", padx=12, pady=(8, 2))
            attempts = self.db.get_question_attempts(q["id"])
            if attempts:
                latest = attempts[0]
                history = (
                    f"Answer history: last answered {latest['selectedAnswer']} "
                    f"({'correct' if latest['isCorrect'] else 'incorrect'})"
                )
                ctk.CTkLabel(
                    detail, text=history, font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY, anchor="w",
                ).pack(anchor="w", padx=12, pady=(0, 4))
            summary = (q.get("explanation") or "").strip()
            if summary:
                ctk.CTkLabel(
                    detail, text=f"Summary: {summary}", font=T.FONT_SMALL,
                    text_color=T.TEXT_MUTED, wraplength=700, justify="left", anchor="w",
                ).pack(anchor="w", padx=12, pady=(8, 4))
            elif not any((opt.get("explanation") or "").strip() for opt in q["options"]):
                ctk.CTkLabel(
                    detail, text=format_option_explanations(q), font=T.FONT_SMALL,
                    text_color=T.TEXT_MUTED, wraplength=700, justify="left", anchor="w",
                ).pack(anchor="w", padx=12, pady=(8, 4))
            citation_text = format_citation_display(q.get("citation"))
            if citation_text:
                ctk.CTkLabel(
                    detail, text=citation_text, font=T.FONT_SMALL,
                    text_color=T.ACCENT, wraplength=700, justify="left", anchor="w",
                ).pack(anchor="w", padx=12, pady=(4, 10))

    def _toggle(self, qid: str):
        self.expanded_id = None if self.expanded_id == qid else qid
        self.refresh()

    def _delete(self, qid: str):
        self.db.delete_question(qid)
        if self.expanded_id == qid:
            self.expanded_id = None
        self.refresh()