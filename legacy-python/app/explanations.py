def format_option_explanations(question: dict) -> str:
    """Build a readable breakdown covering every answer choice."""
    correct = question["correctAnswer"]
    sections: list[str] = []

    summary = (question.get("explanation") or "").strip()
    if summary:
        sections.append(summary)

    for opt in question.get("options") or []:
        label = opt.get("label", "")
        text = (opt.get("text") or "").strip()
        detail = (opt.get("explanation") or "").strip()
        if label == correct:
            header = f"{label}. {text} (Correct)"
            if not detail:
                detail = "This is the best answer."
        else:
            header = f"{label}. {text}"
            if not detail:
                detail = "This option is incorrect."
        sections.append(f"{header}\n{detail}")

    return "\n\n".join(sections)


def options_have_explanations(options: list[dict]) -> bool:
    if len(options) != 5:
        return False
    return all((opt.get("explanation") or "").strip() for opt in options)