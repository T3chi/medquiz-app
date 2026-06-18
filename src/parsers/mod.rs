use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::Regex;
use zip::read::ZipArchive;

pub fn get_file_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "pdf" => Some("pdf"),
        "ppt" | "pptx" => Some("pptx"),
        _ => None,
    }
}

pub fn parse_file(path: &Path) -> anyhow::Result<(String, String)> {
    let file_type = get_file_type(path).ok_or_else(|| {
        anyhow::anyhow!("Unsupported file type. Please upload a PDF or PowerPoint file.")
    })?;
    let text = if file_type == "pdf" {
        parse_pdf(path)?
    } else {
        parse_pptx(path)?
    };
    Ok((text, file_type.to_string()))
}

pub fn parse_pdf(path: &Path) -> anyhow::Result<String> {
    let text = pdf_extract::extract_text(path)
        .map_err(|e| anyhow::anyhow!("Failed to read PDF: {e}"))?
        .trim()
        .to_string();
    if text.is_empty() {
        anyhow::bail!("No text could be extracted from this PDF. It may be image-based or encrypted.");
    }
    Ok(text)
}

pub fn parse_pptx(path: &Path) -> anyhow::Result<String> {
    if path.extension().and_then(|e| e.to_str()) == Some("ppt") {
        anyhow::bail!("Legacy .ppt files are not supported. Please save as .pptx and try again.");
    }
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let slide_re = Regex::new(r"ppt/slides/slide(\d+)\.xml")?;
    let text_re = Regex::new(r"<a:t[^>]*>([^<]*)</a:t>")?;

    let mut slides: Vec<(u32, String)> = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file
            .enclosed_name()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(cap) = slide_re.captures(&name) {
            let num: u32 = cap[1].parse()?;
            let mut xml = String::new();
            file.read_to_string(&mut xml)?;
            let texts: Vec<String> = text_re
                .captures_iter(&xml)
                .filter_map(|c| decode_xml_entities(c.get(1)?.as_str().trim()))
                .collect();
            if !texts.is_empty() {
                slides.push((num, texts.join("\n")));
            }
        }
    }
    if slides.is_empty() {
        anyhow::bail!("No text could be extracted from this PowerPoint file.");
    }
    slides.sort_by_key(|(n, _)| *n);
    let full = slides
        .into_iter()
        .map(|(_, t)| t)
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    Ok(full)
}

fn decode_xml_entities(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    Some(
        text.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'"),
    )
}

pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let truncated = &text[..max_chars];
    if let Some(pos) = truncated.rfind("\n\n") {
        if pos > max_chars * 7 / 10 {
            return format!(
                "{}\n\n[Content truncated for processing...]",
                &truncated[..pos]
            );
        }
    }
    format!("{truncated}\n\n[Content truncated for processing...]")
}