import customtkinter as ctk

from app.config import DEFAULT_APP_SETTINGS
from app.ui import theme as T


class SettingsFrame(ctk.CTkFrame):
    def __init__(self, master, db, **kwargs):
        super().__init__(master, fg_color=T.BG_PRIMARY, **kwargs)
        self.db = db
        self.settings = self.db.get_app_settings()
        self._build()

    def _build(self):
        ctk.CTkLabel(self, text="Settings", font=T.FONT_TITLE, text_color=T.TEXT_PRIMARY).pack(
            anchor="w", padx=32, pady=(28, 4)
        )
        ctk.CTkLabel(
            self, text="Configure your AI provider for question generation.",
            font=T.FONT_BODY, text_color=T.TEXT_SECONDARY,
        ).pack(anchor="w", padx=32, pady=(0, 20))

        card = ctk.CTkFrame(self, fg_color=T.BG_SECONDARY, corner_radius=12, border_width=1, border_color=T.BORDER)
        card.pack(fill="x", padx=32)

        inner = ctk.CTkFrame(card, fg_color="transparent")
        inner.pack(fill="x", padx=24, pady=20)

        presets = ctk.CTkFrame(inner, fg_color="transparent")
        presets.pack(fill="x", pady=(0, 16))
        ctk.CTkButton(
            presets, text="OpenAI Preset", fg_color=T.BG_TERTIARY, hover_color=T.BG_ELEVATED,
            command=self._preset_openai,
        ).pack(side="left", padx=(0, 8))
        ctk.CTkButton(
            presets, text="LM Studio Preset", fg_color=T.BG_TERTIARY, hover_color=T.BG_ELEVATED,
            command=self._preset_lmstudio,
        ).pack(side="left")

        self.api_key = ctk.CTkEntry(inner, placeholder_text="API key", show="*", font=T.FONT_BODY)
        self.api_key.insert(0, self.settings.get("api_key", ""))
        self.api_key.pack(fill="x", pady=4)
        ctk.CTkLabel(
            inner, text="Stored locally only. Use any value for LM Studio.",
            font=T.FONT_SMALL, text_color=T.TEXT_MUTED,
        ).pack(anchor="w", pady=(0, 12))

        ctk.CTkLabel(inner, text="API Base URL", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        self.base_url = ctk.CTkEntry(inner, font=T.FONT_BODY)
        self.base_url.insert(0, self.settings.get("api_base_url", DEFAULT_APP_SETTINGS["api_base_url"]))
        self.base_url.pack(fill="x", pady=(4, 12))

        ctk.CTkLabel(inner, text="Model", font=T.FONT_SMALL, text_color=T.TEXT_SECONDARY).pack(anchor="w")
        self.model = ctk.CTkEntry(inner, font=T.FONT_BODY)
        self.model.insert(0, self.settings.get("model", DEFAULT_APP_SETTINGS["model"]))
        self.model.pack(fill="x", pady=(4, 16))

        self.status = ctk.CTkLabel(inner, text="", font=T.FONT_SMALL, text_color=T.SUCCESS)
        self.status.pack(anchor="w", pady=(0, 8))

        ctk.CTkButton(
            inner, text="Save Settings", font=T.FONT_HEADING, height=40,
            fg_color=T.ACCENT, hover_color=T.ACCENT_HOVER, command=self._save,
        ).pack(fill="x")

        about = ctk.CTkFrame(self, fg_color=T.BG_SECONDARY, corner_radius=12, border_width=1, border_color=T.BORDER)
        about.pack(fill="x", padx=32, pady=16)
        ctk.CTkLabel(
            about,
            text="MedQuiz uses your system native UI and stores all questions locally in SQLite.\n"
                 "Definition · First Order · Second Order difficulty levels supported.",
            font=T.FONT_BODY, text_color=T.TEXT_SECONDARY, justify="left", anchor="w",
        ).pack(padx=24, pady=20)

    def _preset_openai(self):
        self.base_url.delete(0, "end")
        self.base_url.insert(0, "https://api.openai.com/v1")
        self.model.delete(0, "end")
        self.model.insert(0, "gpt-4o-mini")

    def _preset_lmstudio(self):
        self.base_url.delete(0, "end")
        self.base_url.insert(0, "http://localhost:1234/v1")
        self.model.delete(0, "end")
        self.model.insert(0, "local-model")

    def _save(self):
        self.db.save_app_settings({
            "api_key": self.api_key.get().strip(),
            "api_base_url": self.base_url.get().strip(),
            "model": self.model.get().strip(),
        })
        self.status.configure(text="Settings saved successfully")
        self.after(3000, lambda: self.status.configure(text=""))