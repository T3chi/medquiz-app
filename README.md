# MedQuiz

A desktop study app that turns lecture PDFs and PowerPoints into USMLE/COMLEX-style multiple-choice questions. Upload your materials, generate board-style items with AI, practice in quizzes, and track progress over time — all stored locally on your machine.

**Repository:** https://github.com/T3chi/medquiz-app

## History

MedQuiz started as a **Python / CustomTkinter** desktop app focused on NBME-style question generation from uploaded slides. The original version handled PDF/PPTX parsing, SQLite storage, dark-themed UI, and quiz flow with local persistence.

In **v0.2**, the app was rebuilt in **Rust with Dioxus** for a faster, more maintainable desktop experience while preserving the existing SQLite database (questions, attempts, settings, and uploads). The Python implementation remains in `legacy-python/` for reference.

Recent additions include a **Dashboard** with usage analytics and streaks, subject tagging, source citations, per-option explanations, and targeted review quizzes from unanswered or missed questions.

## Features

### Study workflow
- **Upload materials** — PDF and PowerPoint (`.pdf`, `.pptx`, `.ppt`)
- **Generate quizzes** — AI creates NBME-oriented MCQs with configurable count (1–100), difficulty, and exam style (USMLE or COMLEX)
- **Take quizzes** — Practice from your question bank with filters by source, difficulty, and exam style
- **Question bank** — Browse, search, and filter by subject tags; expand items to see options, explanations, and citations

### Question quality
- Vignette + lead-in stems with five options (A–E)
- **Per-option explanations** for correct and incorrect choices
- **Subject tags** (1–3 per question) for search and analytics
- **Source citations** with verified quotes from uploaded lecture text
- Difficulty levels: Definition, First Order, Second Order

### Dashboard & analytics
- Default home view with **study streaks**, accuracy, and daily activity
- Strongest and weakest subjects by performance
- Quick-review quizzes from **unanswered**, **incorrect**, or **mixed** question pools

### Quiz experience
- Selectable/highlightable question stems
- Navigate **previous** and **next** between questions
- Per-question or end-of-quiz answer reveal
- Attempt tracking with last result (correct / incorrect / unanswered)

### Data & privacy
- All questions and progress stored in a local **SQLite** database
- API key used only for question generation (OpenAI or compatible local endpoint e.g. LM Studio)
- Upload directory preference remembered between sessions

## Requirements

- [Rust](https://rustup.rs/) (stable toolchain)
- Windows, macOS, or Linux (desktop target via Dioxus)
- An OpenAI API key **or** a local OpenAI-compatible server (e.g. [LM Studio](https://lmstudio.ai/))

## Run instructions

### Clone and run (recommended)

```powershell
git clone https://github.com/T3chi/medquiz-app.git
cd medquiz-app
cargo run
```

On Windows you can also use:

```powershell
.\run.ps1
```

Or the Python wrapper (invokes `cargo run`):

```powershell
python run.py
```

### First-time Rust setup

If `cargo` is not installed:

```powershell
# Install rustup, then restart your terminal
winget install Rustlang.Rustup
```

### Settings

Open **Settings** in the app and configure:

| Preset | API Base URL | Model |
|--------|----------------|-------|
| OpenAI | `https://api.openai.com/v1` | `gpt-4o-mini` |
| LM Studio | `http://localhost:1234/v1` | `local-model` |

### Database location

Quiz data is stored outside the repo:

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\medquiz-app\medquiz.db` |
| macOS | `~/Library/Application Support/medquiz-app/medquiz.db` |
| Linux | `~/.config/medquiz-app/medquiz.db` |

Upgrading from the Python app reuses this database automatically when paths match.

### Utility: backfill metadata

Backfill subject tags and citations on older questions:

```powershell
cargo run --bin backfill
```

## Project layout

```
medquiz-app/
├── src/              # Rust application (UI, DB, LLM, parsers)
├── assets/           # Stylesheets
├── legacy-python/    # Original CustomTkinter app
├── scripts/          # Dev/maintenance scripts
├── Cargo.toml
└── Dioxus.toml       # Desktop window configuration
```

## Tech stack

- **UI:** Dioxus 0.7 (desktop)
- **Database:** SQLite via `rusqlite`
- **Parsing:** PDF (`pdf-extract`), PPTX (`zip`)
- **AI:** OpenAI-compatible HTTP API (`reqwest`)

## License

No license file is included yet. All rights reserved by the repository owner unless otherwise specified.