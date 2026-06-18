import re


PROFESSOR_PATTERNS = [
    re.compile(
        r"(?:Dr\.|Prof\.|Professor)\s+([A-Z][a-zA-Z]+(?:\s+[A-Z]\.?\s*)?[A-Z][a-zA-Z]+)",
        re.MULTILINE,
    ),
    re.compile(
        r"(?:Dr\.|Prof\.|Professor)\s+([A-Z][a-z]+\s+[A-Z][a-z]+)",
        re.MULTILINE,
    ),
    re.compile(
        r"(?:Presented by|Lecture by|Taught by|Instructor|Faculty|Author)"
        r"[:\s]+(?:Dr\.|Prof\.|Professor)?\s*"
        r"([A-Z][a-zA-Z]+(?:\s+[A-Z]\.?\s*)?[A-Z][a-zA-Z]+)",
        re.IGNORECASE | re.MULTILINE,
    ),
    re.compile(
        r"([A-Z][a-zA-Z]+(?:\s+[A-Z]\.?\s*)?[A-Z][a-zA-Z]+),?\s+(?:MD|PhD|DO|Ph\.D\.)",
        re.MULTILINE,
    ),
    re.compile(
        r"([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\s+(?:MD|PhD|DO)",
        re.MULTILINE,
    ),
]


def extract_professor_name(text: str) -> str | None:
    """Scan title/header regions of source text for instructor names."""
    if not text:
        return None
    # Title slides and headers usually appear in the first portion
    head = text[:4000]
    candidates: list[str] = []
    for pattern in PROFESSOR_PATTERNS:
        for match in pattern.finditer(head):
            name = match.group(1).strip()
            name = re.sub(r"\s+", " ", name)
            if len(name) > 4 and name not in candidates:
                candidates.append(name)
    return candidates[0] if candidates else None


def _normalize_for_match(text: str) -> str:
    text = text.lower()
    text = re.sub(r"[^\w\s]", " ", text)
    return re.sub(r"\s+", " ", text).strip()


def quote_exists_in_source(quote: str, source_text: str) -> bool:
    """Return True if quote is a verbatim (whitespace-normalized) substring of source."""
    if not quote or not source_text:
        return False
    q = _normalize_for_match(quote)
    src = _normalize_for_match(source_text)
    if len(q) < 20:
        return q in src
    # Require substantial contiguous match
    if q in src:
        return True
    # Allow minor truncation: check if 85% of quote words appear in order
    q_words = q.split()
    if len(q_words) < 4:
        return False
    window = max(4, int(len(q_words) * 0.85))
    fragment = " ".join(q_words[:window])
    return fragment in src


def find_verified_quote(quote: str, source_text: str) -> str | None:
    """
    Return a verified quote from source. Uses LLM-provided quote if valid,
    otherwise searches source for the best matching excerpt.
    """
    quote = (quote or "").strip()
    if quote and quote_exists_in_source(quote, source_text):
        return _extract_verbatim_span(quote, source_text) or quote

    if not quote:
        return None

    # Fuzzy: find longest source substring sharing opening words
    q_words = _normalize_for_match(quote).split()
    if len(q_words) < 3:
        return None

    src_norm = _normalize_for_match(source_text)
    for length in range(len(q_words), 2, -1):
        fragment = " ".join(q_words[:length])
        if fragment in src_norm:
            return _extract_verbatim_span_from_normalized(fragment, source_text)
    return None


def _extract_verbatim_span(quote: str, source_text: str) -> str | None:
    """Pull the exact source substring that best matches the quote."""
    quote_norm = _normalize_for_match(quote)
    src_norm = _normalize_for_match(source_text)
    idx = src_norm.find(quote_norm)
    if idx < 0:
        return quote.strip()
    return _extract_verbatim_span_from_normalized(quote_norm, source_text)


def _extract_verbatim_span_from_normalized(fragment: str, source_text: str) -> str:
    """Map normalized match back to a readable excerpt from original source."""
    words = fragment.split()
    if not words:
        return fragment
    pattern = r"\s+".join(re.escape(w) for w in words[: min(12, len(words))])
    match = re.search(pattern, source_text, re.IGNORECASE)
    if not match:
        return fragment
    start = match.start()
    # Extend excerpt to ~quote length
    target_len = max(120, len(fragment) + 40)
    end = min(len(source_text), start + target_len)
    excerpt = source_text[start:end].strip()
    # Trim to sentence boundaries when possible
    if len(excerpt) > 300:
        cut = excerpt[:300]
        last_period = cut.rfind(".")
        if last_period > 80:
            excerpt = cut[: last_period + 1]
        else:
            excerpt = cut + "…"
    return excerpt


def build_citation(
    filename: str,
    professor_name: str | None,
    quote: str,
) -> dict[str, str | None]:
    return {
        "filename": filename,
        "professorName": professor_name,
        "quote": quote,
    }


def format_citation_display(citation: dict | None) -> str:
    if not citation or not citation.get("quote"):
        return ""
    lines = ["Source citation:"]
    if citation.get("filename"):
        lines.append(f"  File: {citation['filename']}")
    prof = citation.get("professorName")
    if prof:
        lines.append(f"  Professor: {prof}")
    else:
        lines.append("  Professor: (not identified in source)")
    lines.append(f'  "{citation["quote"]}"')
    return "\n".join(lines)