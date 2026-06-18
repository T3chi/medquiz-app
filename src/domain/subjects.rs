pub fn normalize_subjects(raw: &serde_json::Value) -> Vec<String> {
    let items: Vec<String> = match raw {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        serde_json::Value::String(s) => s
            .split([',', ';', '/'])
            .map(|p| p.trim().to_string())
            .collect(),
        _ => vec![],
    };

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for item in items {
        let tag = item
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if tag.is_empty() || seen.contains(&tag) {
            continue;
        }
        let tag = if tag.len() > 40 {
            tag[..40].trim_end().to_string()
        } else {
            tag
        };
        seen.insert(tag.clone());
        result.push(tag);
        if result.len() >= 5 {
            break;
        }
    }
    result
}