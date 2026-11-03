"""Captured audit input projection; format decoding does not confer Narrative authority."""
from __future__ import annotations

from dataclasses import dataclass
import io
import os
from pathlib import Path
import shutil
import signal
import time
import stat
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from xml.parsers import expat
import zipfile

import digest as _digest

MAX_INPUT_BYTES = 8 * 1024 * 1024
MAX_CONTAINER_BYTES = 32 * 1024 * 1024
WORD = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
STRICT_WORD = "http://purl.oclc.org/ooxml/wordprocessingml/main"

@dataclass(frozen=True)
class CapturedInput:
    text: str
    raw_sha256: str
    text_sha256: str
    format: str
    extractor: str


def _capture(path: Path, maximum: int) -> bytes:
    entry = path.lstat()
    if not stat.S_ISREG(entry.st_mode):
        raise ValueError(f"input must be a regular non-symlink file: {path}")
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    with os.fdopen(descriptor, "rb") as source:
        held = os.fstat(source.fileno())
        if not stat.S_ISREG(held.st_mode) or (held.st_dev, held.st_ino) != (entry.st_dev, entry.st_ino):
            raise ValueError(f"input identity changed during open: {path}")
        if held.st_size > maximum:
            raise ValueError(f"input exceeds {maximum} bytes: {path}")
        raw = source.read(maximum + 1)
        if len(raw) != held.st_size or len(raw) > maximum:
            raise ValueError(f"input changed size during bounded read: {path}")
    return raw


def _bounded_text(text: str) -> str:
    if len(text.encode("utf-8")) > MAX_INPUT_BYTES:
        raise ValueError("extracted text exceeds its 8 MiB budget")
    if not text.strip():
        raise ValueError("input contains no extractable text")
    return text


def _xml(raw: bytes) -> ET.Element:
    if len(raw) > MAX_INPUT_BYTES:
        raise ValueError("XML part exceeds its byte budget")
    parser = expat.ParserCreate()
    def unsupported(*_args: object) -> None:
        raise ValueError("DTD and external entity declarations are unsupported")
    parser.StartDoctypeDeclHandler = unsupported
    parser.ExternalEntityRefHandler = unsupported
    parser.Parse(raw, True)
    return ET.fromstring(raw)


def _append_paragraph(out: list[str], label: str, text: str, used: int) -> int:
    line = f"[{label}] {text}" if text.strip() else ""
    used += len(line.encode("utf-8")) + 1
    if used > MAX_INPUT_BYTES:
        raise ValueError("tagged document text exceeds its byte budget")
    out.append(line)
    return used


def _fdx(raw: bytes) -> str:
    root = _xml(raw)
    if root.tag != "FinalDraft":
        raise ValueError("FDX requires FinalDraft root")
    contents = root.findall("Content")
    if len(contents) != 1:
        raise ValueError("FDX requires exactly one Content body")
    allowed = {"Scene Heading", "Character", "Dialogue", "Action", "General", "Parenthetical", "Transition"}
    out: list[str] = []
    used = 0
    for paragraph in contents[0]:
        if paragraph.tag != "Paragraph":
            raise ValueError(f"unsupported FDX body element: {paragraph.tag}")
        kind = paragraph.get("Type")
        if kind not in allowed:
            raise ValueError(f"unsupported or missing FDX Paragraph Type: {kind}")
        parts: list[str] = []
        for child in paragraph:
            if child.tag != "Text" or list(child):
                raise ValueError(f"unsupported FDX paragraph content: {child.tag}")
            parts.append(child.text or "")
        text = "".join(parts)
        used = _append_paragraph(out, kind, text, used)
    return _bounded_text("\n".join(out) + "\n")


def _part(archive: zipfile.ZipFile, name: str, required: bool) -> bytes | None:
    entries = [entry for entry in archive.infolist() if entry.filename == name]
    if not entries and not required:
        return None
    if len(entries) != 1 or entries[0].is_dir():
        raise ValueError(f"DOCX requires an unambiguous regular {name} part")
    if entries[0].file_size > MAX_INPUT_BYTES:
        raise ValueError(f"DOCX {name} exceeds its uncompressed budget")
    with archive.open(entries[0]) as source:
        raw = source.read(MAX_INPUT_BYTES + 1)
    if len(raw) > MAX_INPUT_BYTES or len(raw) != entries[0].file_size:
        raise ValueError(f"DOCX {name} exceeds its actual read budget")
    return raw


def _docx(raw: bytes) -> str:
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        document = _xml(_part(archive, "word/document.xml", True))
        namespace = next((uri for uri in (WORD, STRICT_WORD) if document.tag == f"{{{uri}}}document"), None)
        if namespace is None:
            raise ValueError("DOCX requires a WordprocessingML document root")
        q = lambda name: f"{{{namespace}}}{name}"
        bodies = document.findall(q("body"))
        if len(bodies) != 1:
            raise ValueError("DOCX requires exactly one body")
        body = bodies[0]
        if any(element.tag in {q(name) for name in ("ins", "del", "moveFrom", "moveTo", "altChunk")} for element in body.iter()):
            raise ValueError("DOCX tracked revisions/embedded body content require resolution")
        style_names: dict[str, str] = {}
        style_raw = _part(archive, "word/styles.xml", False)
        if style_raw is not None:
            styles = _xml(style_raw)
            if styles.tag != q("styles"):
                raise ValueError("DOCX style namespace differs from its body")
            for style in styles.findall(q("style")):
                sid = style.get(q("styleId"))
                if not sid or not sid.strip() or sid in style_names:
                    raise ValueError("DOCX style IDs must be nonempty and unique")
                name = style.find(q("name"))
                label = name.get(q("val")) if name is not None else sid
                if not label or not label.strip() or any(character in label for character in "\r\n[]"):
                    raise ValueError("DOCX style label cannot be represented in tagged text")
                style_names[sid] = label
        out: list[str] = []
        used = 0
        for paragraph in body.iter(q("p")):
            parts: list[str] = []
            def gather(element: ET.Element) -> None:
                for child in element:
                    if child.tag == q("p"):
                        continue  # Nested paragraphs are visited once by the body iterator.
                    if child.tag == q("t"):
                        if list(child): raise ValueError("DOCX text element cannot contain child elements")
                        parts.append(child.text or "")
                    elif child.tag == q("tab"): parts.append("\t")
                    elif child.tag in (q("br"), q("cr")): parts.append("\n")
                    else: gather(child)
            gather(paragraph)
            text = "".join(parts)
            style = paragraph.find(f'{q("pPr")}/{q("pStyle")}')
            sid = style.get(q("val")) if style is not None else None
            if style is not None and (not sid or not sid.strip()): raise ValueError("DOCX paragraph style requires a value")
            label = style_names.get(sid, sid) if sid else "Paragraph"
            if any(character in label for character in "\r\n[]"):
                raise ValueError("DOCX paragraph style cannot be represented in tagged text")
            used = _append_paragraph(out, label, text, used)
        return _bounded_text("\n".join(out) + "\n")


def _run_pdftotext(executable: str, source: Path, output: Path, errors: Path) -> int:
    if os.name != "posix":
        raise ValueError("pdftotext process-group custody requires Unix")
    with errors.open("xb") as sink:
        child = subprocess.Popen([executable, "-layout", str(source), str(output)], stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=sink, start_new_session=True)
        try:
            deadline = time.monotonic() + 60
            while child.poll() is None:
                if time.monotonic() >= deadline: raise ValueError("pdftotext timed out")
                for path, maximum in ((output, MAX_INPUT_BYTES), (errors, 64 * 1024)):
                    try: entry = path.lstat()
                    except FileNotFoundError: continue
                    if not stat.S_ISREG(entry.st_mode) or entry.st_size > maximum:
                        raise ValueError("pdftotext output exceeded its regular-file budget")
                time.sleep(0.02)
            for path, maximum in ((output, MAX_INPUT_BYTES), (errors, 64 * 1024)):
                entry = path.lstat()
                if not stat.S_ISREG(entry.st_mode) or entry.st_size > maximum:
                    raise ValueError("pdftotext terminal output exceeded its regular-file budget")
            try: os.killpg(child.pid, 0)
            except ProcessLookupError: pass
            else: raise ValueError("pdftotext exited with residual owned-group processes")
            return child.returncode
        except BaseException as original:
            try:
                try: os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                child.wait()
            except BaseException as cleanup:
                raise RuntimeError(f"{original}; pdftotext cleanup failed; outcome unknown: {cleanup}") from original
            raise


def _pdf(raw: bytes) -> tuple[str, str]:
    executable = shutil.which("pdftotext")
    if executable:
        with tempfile.TemporaryDirectory(prefix="sts-audit-pdf-") as directory:
            root = Path(directory)
            source = root / "source.pdf"
            output = root / "text.txt"
            errors = root / "stderr"
            source.write_bytes(raw)
            status = _run_pdftotext(executable, source, output, errors)
            if status:
                with errors.open("rb") as details:
                    message = details.read(64 * 1024).decode("utf-8", errors="replace")
                raise ValueError(f"pdftotext failed ({status}): {message}")
            return _bounded_text(_capture(output, MAX_INPUT_BYTES).decode("utf-8")), "pdftotext"
    try:
        from pypdf import PdfReader
    except ImportError as error:
        raise ValueError("PDF extraction requires pdftotext or pypdf") from error
    reader = PdfReader(io.BytesIO(raw))
    parts: list[str] = []
    size = 0
    for page in reader.pages:
        text = page.extract_text()
        if text is None:
            raise ValueError("PDF page has no extractable text; partial extraction is refused")
        size += len(text.encode("utf-8")) + 2
        if size > MAX_INPUT_BYTES:
            raise ValueError("PDF extracted text exceeds its byte budget")
        parts.append(text)
    return _bounded_text("\n\n".join(parts)), "pypdf"


def capture_input(path: Path) -> CapturedInput:
    suffix = path.suffix.lower()
    if suffix not in {".md", ".txt", ".fountain", ".spmd", ".fdx", ".docx", ".pdf"}:
        raise ValueError(f"unsupported input type: {suffix or '<none>'}")
    raw = _capture(path, MAX_CONTAINER_BYTES if suffix in {".pdf", ".docx"} else MAX_INPUT_BYTES)
    if suffix == ".fdx": text, extractor = _fdx(raw), "fdx-body"
    elif suffix == ".docx": text, extractor = _docx(raw), "docx-body"
    elif suffix == ".pdf": text, extractor = _pdf(raw)
    else: text, extractor = _bounded_text(raw.decode("utf-8")), "exact-utf8"
    return CapturedInput(text, _digest.sha256_bytes(raw), _digest.sha256_bytes(text.encode("utf-8")), suffix, extractor)


def extract_fdx(path: Path) -> str:
    if path.suffix.lower() != ".fdx": raise ValueError("FDX path required")
    return capture_input(path).text


def extract_docx(path: Path) -> str:
    if path.suffix.lower() != ".docx": raise ValueError("DOCX path required")
    return capture_input(path).text
