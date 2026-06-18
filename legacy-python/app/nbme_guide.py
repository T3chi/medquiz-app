"""
NBME Item-Writing Guide (6th ed.) compliance rules for one-best-answer (A-type) items.
Reference: https://www.nbme.org/sites/default/files/2021-02/NBME_Item%20Writing%20Guide_R_6.pdf
"""

import re

# Approved lead-in stems (Appendix B) — use one per item, matched to testing point
APPROVED_LEAD_INS = [
    "Which of the following is the most likely diagnosis?",
    "Which of the following is the most appropriate next step in management?",
    "Which of the following is the most appropriate pharmacotherapy?",
    "Which of the following is the most appropriate initial diagnostic study?",
    "Which of the following is the most appropriate next step in evaluation?",
    "Which of the following is the most likely underlying cause of this patient's condition?",
    "Which of the following is the most likely mechanism of action of this drug?",
    "Which of the following is the most likely explanation for this patient's condition?",
    "Which of the following pathogens is the most likely cause of this patient's condition?",
    "Which of the following findings is most consistent with the underlying diagnosis?",
    "Which of the following factors in this patient's history most increased the risk for developing this condition?",
    "Which of the following is the most appropriate preventive measure?",
    "Which of the following best describes this pathophysiologic process?",
    "Which of the following is the most accurate statement regarding this condition?",
]

FORBIDDEN_LEAD_IN_PATTERNS = [
    r"\bexcept\b",
    r"\bnot\b",
    r"\bleast likely\b",
    r"\bincorrect\b",
    r"\bfalse\b",
    r"\ball of the above\b",
    r"\bnone of the above\b",
    r":\s*$",  # open-ended colon lead-in
]

FORBIDDEN_OPTION_PHRASES = [
    "all of the above",
    "none of the above",
    "both a and b",
    "a and b",
    "a, b, and c",
]

FORBIDDEN_VAGUE_TERMS = re.compile(
    r"\b(usually|frequently|often|may|could be|is associated with|is useful for|is important)\b",
    re.IGNORECASE,
)

ABSOLUTE_TERMS = re.compile(r"\b(always|never|only|must|cannot|impossible)\b", re.IGNORECASE)

NBME_SYSTEM_RULES = """
NBME ONE-BEST-ANSWER (A-TYPE) ITEM RULES — MANDATORY COMPLIANCE

FORMAT (Chapter 2):
- Use only one-best-answer A-type items with exactly 5 options (A through E).
- Structure each item as: VIGNETTE (clinical or experimental scenario) + LEAD-IN (single closed question) + OPTION SET.
- The keyed answer is the single BEST option; distractors may be partially correct but less correct than the key.
- Do NOT use true-false, K-type, X-type, or "select all that apply" formats.

RULE 1 — Important testing points (Chapter 5):
- Test important, clinically relevant concepts from the source material, not trivial facts or esoterica ("zebras").
- Focus on common or potentially catastrophic problems learners must recognize.

RULE 2 — Application of knowledge (Chapter 5–6):
- DEFAULT: Assess APPLICATION of knowledge, not isolated fact recall.
- Use clinical vignettes that require integrating findings to reach a diagnosis, next step, mechanism, or management decision.
- Present information in clinically natural order: patient comes with symptoms → provider evaluates → decides.
- Do NOT write backward items (name a disease first, then ask for findings).
- For foundational/basic science items: still require applying a science principle to a clinical or experimental context.

RULE 3 — Focused, closed lead-in + cover-the-options rule (Chapters 5–6):
- End the vignette with ONE clear, closed lead-in question.
- The lead-in must be focused (e.g., "most likely diagnosis", "most appropriate next step") — never open-ended ("The diagnosis is:").
- A knowledgeable examinee should be able to answer correctly after reading ONLY the vignette and lead-in, without seeing options.
- Use positively phrased lead-ins. Avoid "EXCEPT", "NOT", "least likely", or negative wording.
- Avoid vague lead-ins: "is associated with", "is useful for", "is important".
- Select lead-ins from standard NBME task-competency phrasing (diagnosis, management, mechanism, diagnostic study, risk factor, etc.).

VIGNETTE TEMPLATE (Chapter 6) — include relevant elements in this order when applicable:
1. Age and gender (e.g., "A 45-year-old woman")
2. Site of care (e.g., emergency department, office)
3. Chief concern and duration
4. Pertinent history (PMH, medications, family/social history)
5. Physical examination findings
6. Diagnostic study results
7. Initial treatment and subsequent findings (if relevant)
- Include only information needed for the testing point; avoid gratuitous "window dressing" and misleading red herrings.
- Use precise, unambiguous language. Avoid "claims", "allegedly", or unreliable-history framing.

RULE 4 — Homogeneous, plausible options (Chapters 2, 5):
- ALL five options must address the SAME dimension as the lead-in (all diagnoses, all drugs, all tests, all mechanisms, etc.).
- Options must be rank-orderable from least to most correct along a single continuum.
- Distractors must be plausible in the clinical context — not absurd or easily eliminated.
- Do NOT mix categories (e.g., diagnoses mixed with pathogens, or treatments mixed with diagnostic tests).
- Keep options CONCISE and PARALLEL in grammatical structure and length.
- All clinical data belongs in the vignette, NOT in the options.

RULE 5 — Eliminate technical item flaws (Chapter 3):
NEVER include these flaws:
- Grammatical cues (options that don't complete the lead-in grammatically)
- Word repetition / "clang" clues (correct answer echoes distinctive words from the vignette)
- Longest or most detailed option as correct answer (correct option must not stand out by length)
- Absolute terms in some options only ("always", "never") while others use hedged language
- Vague frequency terms ("often", "usually", "frequently", "may", "could be")
- "None of the above" or "All of the above"
- Nonparallel option structures
- Collectively exhaustive option subsets that cue the answer
- Inconsistent numeric formats in options
- Extraneous complexity in the stem (Roman numeral ranking tasks, multi-step instructions unrelated to clinical reasoning)
- Options longer than one brief phrase or short sentence each

EXPLANATION REQUIREMENTS:
- Explain why the keyed answer is the BEST (most correct) option.
- Briefly explain why each distractor is less correct (not merely "wrong").
- Teach the underlying concept; align with NBME educational purpose.
"""


DIFFICULTY_NBME_GUIDANCE = {
    "definition": """
COGNITION LEVEL: Foundational recall WITH proper NBME structure.
- Use shorter vignettes or focused science contexts when appropriate.
- Lead-ins may target definitions, mechanisms, or core facts (e.g., "Which of the following best describes…", "Which of the following is the most accurate statement…").
- Still use one-best-answer format with homogeneous options and cover-the-options compliance.
- Even recall items must have a closed lead-in and plausible distractors — never bare "fact + list" items.
""",
    "first_order": """
COGNITION LEVEL: Application of knowledge — single-step clinical reasoning.
- Require a clinical vignette with sufficient context to apply one major concept from the source material.
- Lead-ins: diagnosis, next diagnostic step, initial management, or mechanism.
- Examinee must interpret presented findings and reach one conclusion.
""",
    "second_order": """
COGNITION LEVEL: Application of knowledge — multi-step integration.
- Require a vignette demanding synthesis of 2+ concepts (differential diagnosis, pathophysiology + management, complication recognition).
- May include some extraneous findings the examinee must filter out.
- Lead-ins: most likely diagnosis among close differentials, most appropriate next step after partial workup, underlying cause/mechanism.
""",
}


def validate_question(stem: str, options: list[dict], correct_answer: str) -> list[str]:
    """Return a list of NBME compliance violations for a generated question."""
    issues: list[str] = []

    if len(options) != 5:
        issues.append(f"Must have exactly 5 options (found {len(options)}).")

    stem_lower = stem.lower()
    for pattern in FORBIDDEN_LEAD_IN_PATTERNS:
        if re.search(pattern, stem_lower):
            issues.append(f"Lead-in contains forbidden pattern: {pattern}")

    if FORBIDDEN_VAGUE_TERMS.search(stem):
        issues.append("Vignette or lead-in contains vague NBME-prohibited phrasing.")

    if "?" not in stem:
        issues.append("Stem must include a clear lead-in question ending with '?'.")
    elif not re.search(
        r"which of the following|what is the most|this patient most likely",
        stem_lower,
    ):
        issues.append("Lead-in should use standard NBME closed-question phrasing.")

    option_texts = [o["text"].strip() for o in options]
    option_lower = [t.lower() for t in option_texts]

    for phrase in FORBIDDEN_OPTION_PHRASES:
        for text in option_lower:
            if phrase in text:
                issues.append(f"Option contains forbidden phrase: '{phrase}'")

    lengths = [len(t) for t in option_texts]
    correct_idx = next(
        (i for i, o in enumerate(options) if o["label"] == correct_answer), -1
    )
    if correct_idx >= 0:
        correct_len = lengths[correct_idx]
        others = [l for i, l in enumerate(lengths) if i != correct_idx]
        if others and correct_len > max(others) * 1.5:
            issues.append("Correct option is substantially longer than distractors (clang/length cue).")

    abs_counts = [len(ABSOLUTE_TERMS.findall(t)) for t in option_texts]
    if any(c > 0 for c in abs_counts) and not all(c > 0 for c in abs_counts):
        issues.append("Absolute terms appear in only some options (testwise cue).")

    for text in option_texts:
        if FORBIDDEN_VAGUE_TERMS.search(text):
            issues.append(f"Option contains vague terms: '{text[:60]}…'")
        if len(text) > 120:
            issues.append("Options must be concise (≤120 chars each).")

    # Clang clue: significant word overlap between stem and correct option
    if correct_idx >= 0:
        stem_words = set(re.findall(r"[a-z]{5,}", stem_lower))
        correct_words = set(re.findall(r"[a-z]{5,}", option_lower[correct_idx]))
        overlap = stem_words & correct_words
        if len(overlap) >= 2:
            issues.append(f"Possible clang clue — stem and correct answer share terms: {overlap}")

    return issues