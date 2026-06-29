# MedQuiz

A desktop study app that turns lecture PDFs and PowerPoints into USMLE/COMLEX-style multiple-choice questions. Upload your materials, generate board-style items with AI, practice in quizzes, progress through adaptive Learn sessions, and track progress over time — all stored locally on your machine.

**Repository:** https://github.com/T3chi/medquiz-app

> **README last synced:** 2026-06-19 — reflects v0.3 UI (Home hub, Learn mode, keyboard shortcuts, visual polish). See also [docs/KEYBOARD_SHORTCUTS.md](docs/KEYBOARD_SHORTCUTS.md).

## History

MedQuiz started as a **Python / CustomTkinter** desktop app focused on NBME-style question generation from uploaded slides. The original version handled PDF/PPTX parsing, SQLite storage, dark-themed UI, and quiz flow with local persistence.

In **v0.2**, the app was rebuilt in **Rust with Dioxus** for a faster, more maintainable desktop experience while preserving the existing SQLite database (questions, attempts, settings, and uploads). The Python implementation remains in `legacy-python/` for reference.

**v0.3** refactors the desktop UI into modular screens and streamlines the study workflow:

- **Home** replaces the old Dashboard as the default landing page and session launcher
- **Materials** consolidates upload, generation, and file management (formerly spread across separate flows)
- **Learn mode** adds six progressive study modalities per concept with per-question mastery tracking
- **Onboarding wizard** walks new users through AI setup, upload, and first session
- **Daily goals** and a progress ring on Home track questions answered each day
- UI code moves from a single `ui.rs` into `src/ui/` (sidebar, home, materials, bank, quiz, learn, settings)

Earlier v0.2 additions — subject tagging, source citations, per-option explanations, analytics, and targeted review quizzes — are preserved and integrated into the new Home workflow.

## Features

### Navigation & shell
- **Sidebar** — Home, Materials, Question Bank, Settings
- **Session-aware nav** — sidebar locks while a quiz or Learn session is active; exit confirms via dialog
- **API status pill** — shows whether an AI provider is configured (OpenAI or local endpoint)
- **First-run onboarding** — three-step welcome flow on Home (AI → study loop → get started)

### Home (study command center)
- **Daily goal ring** — visual progress toward your configurable daily question target (Settings)
- **Start session** tabs:
  - **Smart review** — unanswered, incorrect, or mixed pools from your bank
  - **Practice** — filtered quiz (exam style, difficulty, optional source file)
  - **Learn** — adaptive modality progression (see below)
- **Analytics** — streaks, accuracy, daily activity chart, strongest/weakest subjects
- **Clickable subject rows** — launch a 10-question practice set for any ranked subject
- **Empty-state CTA** — directs new users to Materials when no questions exist

### Learn mode
Progress through **six modalities** per concept; mastery is stored in SQLite (`learn_mastery` table):

| Level | Modality | Description |
|-------|----------|-------------|
| 1 | Multiple Choice | Standard MCQ from your question bank |
| 2 | Matching | Match labels to descriptions |
| 3 | Short Answer | Free-text response graded by keyword overlap |
| 4 | Analogy Completion | Fill-in-the-blank analogy prompts |
| 5 | Create Analogy | Write a 2–4 sentence everyday-life analogy |
| 6 | Relationship Arrows | Classify pairs as increase (↑), decrease (↓), or associated (↔) |

- **Two correct answers** in a row advance to the next level; a wrong answer drops one level
- Session summary shows items completed, correct count, and level-ups
- Concepts are selected from your bank (prioritizing lower mastery levels)

### Materials
- **Upload** PDF and PowerPoint (`.pdf`, `.pptx`, `.ppt`)
- **Step-based wizard** — pick file → configure generation → generate → start quiz or generate more
- **Generate quizzes** — AI creates NBME-oriented MCQs with configurable count (1–100), difficulty, and exam style (USMLE or COMLEX)
- Remembers last upload directory between sessions

### Question bank
- Browse, search, and filter by subject tags or stem text
- **Filter chips** — All, Unanswered, Incorrect, Mastered (Learn level 6)
- **Sort** — Recent, Most missed, Never seen
- **Bulk select** — checkbox per card; launch a quiz from selected items
- **Status pills** — New, Missed, Practiced, Mastered (color-coded)
- **Learn level badge** per question (L1–L6)
- Expand items to see options, explanations, and citations
- Delete questions with confirmation

### Quiz experience
- Selectable/highlightable question stems
- Navigate **previous** and **next** between questions (buttons or **←** / **→**)
- **Keyboard shortcuts** — `1`–`5` for options, `Enter` to submit/advance, `Esc` to exit
- Number hints on each answer option
- Per-question or end-of-quiz answer reveal
- Attempt tracking with last result (correct / incorrect / unanswered)
- Retry incorrect questions at end of session

### Keyboard shortcuts

| Context | Keys | Action |
|---------|------|--------|
| Quiz | `1` – `5` | Select A – E |
| Quiz | `Enter` | Submit / advance / finish |
| Quiz | `←` `→` | Previous / next question |
| Quiz / Learn | `Esc` | Exit (confirmation) |
| Learn (MCQ) | `1` – `5`, `Enter` | Select / check / next |
| Dialogs | `Esc` / `Enter` | Cancel / confirm |

Full details: [docs/KEYBOARD_SHORTCUTS.md](docs/KEYBOARD_SHORTCUTS.md). A summary table also appears under **Settings → Keyboard shortcuts**.

### Visual design

- **Dark theme** with CSS variables in `assets/style.css` (blue accent, semantic success/error/warning)
- Focus rings on interactive controls for keyboard accessibility
- Semantic feedback on quiz options (selected, correct, incorrect) and bank status pills
- Subtle page fade-in and card elevation; activity chart and daily goal ring on Home

### Question quality
- Vignette + lead-in stems with five options (A–E)
- **Per-option explanations** for correct and incorrect choices
- **Subject tags** (1–3 per question) for search and analytics
- **Source citations** with verified quotes from uploaded lecture text
- Difficulty levels: Definition, First Order, Second Order

### Data & privacy
- All questions, mastery, and progress stored in a local **SQLite** database
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

Also set your **daily question goal** and default quiz preferences (exam style, difficulty, answer timing).

### Database location

Quiz data is stored outside the repo:

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\medquiz-app\medquiz.db` |
| macOS | `~/Library/Application Support/medquiz-app/medquiz.db` |
| Linux | `~/.config/medquiz-app/medquiz.db` |

Upgrading from the Python app reuses this database automatically when paths match. The v0.3 schema adds a `learn_mastery` table; existing databases migrate on first launch.

### Utility: backfill metadata

Backfill subject tags and citations on older questions:

```powershell
cargo run --bin backfill
```

## Project layout

```
medquiz-app/
├── src/
│   ├── ui/           # Dioxus screens (home, materials, bank, quiz, learn, settings, sidebar)
│   ├── domain/       # Business logic (analytics, learn grading, NBME generation, etc.)
│   ├── db.rs         # SQLite persistence
│   ├── models.rs     # Shared types
│   ├── services.rs   # Orchestration (quiz generation, etc.)
│   └── main.rs       # Desktop launcher
├── assets/           # Global stylesheet (dark theme)
├── docs/             # User guides (keyboard shortcuts)
├── legacy-python/    # Original CustomTkinter app
├── scripts/          # Dev/maintenance scripts
├── Cargo.toml
└── Dioxus.toml       # Desktop window configuration
```

## Tech stack

- **UI:** Dioxus 0.7 (desktop), modular `src/ui/` components
- **Database:** SQLite via `rusqlite` (questions, attempts, learn mastery, preferences)
- **Parsing:** PDF (`pdf-extract`), PPTX (`zip`)
- **AI:** OpenAI-compatible HTTP API (`reqwest`)

## License

No license file is included yet. All rights reserved by the repository owner unless otherwise specified.