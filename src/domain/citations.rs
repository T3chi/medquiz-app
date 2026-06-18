use regex::Regex;

use crate::models::Citation;

fn normalize_for_match(text: &str) -> String {
    let re = Regex::new(r"[^\w\s]").unwrap();
    re.replace_all(&text.to_lowercase(), " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn extract_professor_name(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    let head = &text[..text.len().min(4000)];
    let patterns = [
        r"(?i)(?:Dr\.|Prof\.|Professor)\s+([A-Z][a-zA-Z]+(?:\s+[A-Z]\.?\s*)?[A-Z][a-zA-Z]+)",
        r"(?i)(?:Presented by|Lecture by|Taught by|Instructor|Faculty|Author)[:\s]+(?:Dr\.|Prof\.|Professor)?\s*([A-Z][a-zA-Z]+(?:\s+[A-Z]\.?\s*)?[A-Z][a-zA-Z]+)",
        r"([A-Z][a-zA-Z]+(?:\s+[A-Z]\.?\s*)?[A-Z][a-zA-Z]+),?\s+(?:MD|PhD|DO|Ph\.D\.)",
    ];
    for pat in patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(cap) = re.captures(head) {
                if let Some(name) = cap.get(1) {
                    let name = name.as_str().split_whitespace().collect::<Vec<_>>().join(" ");
                    if name.len() > 4 {
                        return Some(name);
                    }
                }
            }
        }
    }
    None
}

pub fn quote_exists_in_source(quote: &str, source_text: &str) -> bool {
    if quote.is_empty() || source_text.is_empty() {
        return false;
    }
    let q = normalize_for_match(quote);
    let src = normalize_for_match(source_text);
    if q.len() < 20 {
        return src.contains(&q);
    }
    if src.contains(&q) {
        return true;
    }
    let q_words: Vec<_> = q.split_whitespace().collect();
    if q_words.len() < 4 {
        return false;
    }
    let window = (q_words.len() as f64 * 0.85).max(4.0) as usize;
    let fragment = q_words[..window].join(" ");
    src.contains(&fragment)
}

pub fn find_verified_quote(quote: &str, source_text: &str) -> Option<String> {
    let quote = quote.trim();
    if !quote.is_empty() && quote_exists_in_source(quote, source_text) {
        return Some(extract_verbatim_span(quote, source_text).unwrap_or_else(|| quote.to_string()));
    }
    if quote.is_empty() {
        return None;
    }
    let normalized = normalize_for_match(quote);
    let q_words: Vec<_> = normalized.split_whitespace().collect();
    if q_words.len() < 3 {
        return None;
    }
    let src_norm = normalize_for_match(source_text);
    for length in (3..=q_words.len()).rev() {
        let fragment = q_words[..length].join(" ");
        if src_norm.contains(&fragment) {
            return Some(extract_from_normalized(&fragment, source_text));
        }
    }
    None
}

fn extract_verbatim_span(quote: &str, source_text: &str) -> Option<String> {
    let quote_norm = normalize_for_match(quote);
    let src_norm = normalize_for_match(source_text);
    if src_norm.find(&quote_norm).is_some() {
        return Some(extract_from_normalized(&quote_norm, source_text));
    }
    Some(quote.trim().to_string())
}

fn extract_from_normalized(fragment: &str, source_text: &str) -> String {
    let words: Vec<_> = fragment.split_whitespace().collect();
    if words.is_empty() {
        return fragment.to_string();
    }
    let take = words.len().min(12);
    let pattern = words[..take]
        .iter()
        .map(|w| regex::escape(w))
        .collect::<Vec<_>>()
        .join(r"\s+");
    if let Ok(re) = Regex::new(&format!("(?i){pattern}")) {
        if let Some(m) = re.find(source_text) {
            let start = m.start();
            let target_len = 120.max(fragment.len() + 40);
            let end = (start + target_len).min(source_text.len());
            let mut excerpt = source_text[start..end].trim().to_string();
            if excerpt.len() > 300 {
                let cut = &excerpt[..300];
                if let Some(pos) = cut.rfind('.') {
                    if pos > 80 {
                        excerpt = cut[..=pos].to_string();
                    } else {
                        excerpt = format!("{}…", cut);
                    }
                } else {
                    excerpt = format!("{}…", cut);
                }
            }
            return excerpt;
        }
    }
    fragment.to_string()
}

pub fn build_citation(filename: &str, professor_name: Option<String>, quote: String) -> Citation {
    Citation {
        filename: if filename.is_empty() {
            None
        } else {
            Some(filename.to_string())
        },
        professor_name,
        quote: Some(quote),
    }
}

pub fn format_citation_display(citation: &Option<Citation>) -> String {
    let Some(c) = citation else {
        return String::new();
    };
    let Some(quote) = &c.quote else {
        return String::new();
    };
    if quote.is_empty() {
        return String::new();
    }
    let mut lines = vec!["Source citation:".to_string()];
    if let Some(f) = &c.filename {
        lines.push(format!("  File: {f}"));
    }
    if let Some(p) = &c.professor_name {
        lines.push(format!("  Professor: {p}"));
    } else {
        lines.push("  Professor: (not identified in source)".to_string());
    }
    lines.push(format!("  \"{quote}\""));
    lines.join("\n")
}