import re
import zipfile
from pathlib import Path
from xml.etree import ElementTree


def get_file_type(file_path: str | Path) -> str | None:
    ext = Path(file_path).suffix.lower()
    if ext == ".pdf":
        return "pdf"
    if ext in (".ppt", ".pptx"):
        return "pptx"
    return None


def parse_file(file_path: str | Path) -> tuple[str, str]:
    path = Path(file_path)
    file_type = get_file_type(path)
    if not file_type:
        raise ValueError(
            "Unsupported file type. Please upload a PDF or PowerPoint (.ppt/.pptx) file."
        )
    if file_type == "pdf":
        return parse_pdf(path), file_type
    return parse_pptx(path), file_type


def parse_pdf(path: Path) -> str:
    from pypdf import PdfReader

    reader = PdfReader(str(path))
    pages = []
    for page in reader.pages:
        text = page.extract_text()
        if text:
            pages.append(text.strip())
    text = "\n\n".join(pages).strip()
    if not text:
        raise ValueError(
            "No text could be extracted from this PDF. It may be image-based or encrypted."
        )
    return text


def parse_pptx(path: Path) -> str:
    if path.suffix.lower() == ".ppt":
        raise ValueError(
            "Legacy .ppt files are not supported. Please save as .pptx and try again."
        )

    slide_texts: list[str] = []
    with zipfile.ZipFile(path, "r") as zf:
        slide_files = sorted(
            (n for n in zf.namelist() if re.match(r"ppt/slides/slide\d+\.xml$", n)),
            key=lambda n: int(re.search(r"slide(\d+)", n).group(1)),  # type: ignore[union-attr]
        )
        if not slide_files:
            raise ValueError("No slides found in this PowerPoint file.")
        for slide_file in slide_files:
            xml = zf.read(slide_file).decode("utf-8")
            texts = _extract_text_from_xml(xml)
            if texts:
                slide_texts.append("\n".join(texts))

    full_text = "\n\n---\n\n".join(slide_texts).strip()
    if not full_text:
        raise ValueError("No text could be extracted from this PowerPoint file.")
    return full_text


def _extract_text_from_xml(xml: str) -> list[str]:
    texts: list[str] = []
    for match in re.finditer(r"<a:t[^>]*>([^<]*)</a:t>", xml):
        text = _decode_xml_entities(match.group(1).strip())
        if text:
            texts.append(text)
    return texts


def _decode_xml_entities(text: str) -> str:
    return (
        text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", '"')
        .replace("&apos;", "'")
    )


def truncate_text(text: str, max_chars: int = 12000) -> str:
    if len(text) <= max_chars:
        return text
    truncated = text[:max_chars]
    last_para = truncated.rfind("\n\n")
    if last_para > max_chars * 0.7:
        return truncated[:last_para] + "\n\n[Content truncated for processing...]"
    return truncated + "\n\n[Content truncated for processing...]"