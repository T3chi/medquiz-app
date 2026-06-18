import re


def normalize_subjects(raw) -> list[str]:
    """Normalize AI-provided subject tags into lowercase, deduplicated labels."""
    if raw is None:
        return []
    if isinstance(raw, str):
        raw = [part.strip() for part in re.split(r"[,;/]", raw)]
    if not isinstance(raw, list):
        return []

    seen: set[str] = set()
    result: list[str] = []
    for item in raw:
        tag = re.sub(r"\s+", " ", str(item).strip().lower())
        tag = tag.strip("\"'")
        if not tag or tag in seen:
            continue
        if len(tag) > 40:
            tag = tag[:40].rstrip()
        seen.add(tag)
        result.append(tag)
        if len(result) >= 5:
            break
    return result