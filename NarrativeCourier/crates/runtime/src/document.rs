//! Bounded, domain-neutral text extraction from one captured regular document.

mod docx;

use crate::{Error, Result};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

const MAX_DOCUMENT_BYTES: u64 = 25 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: usize = 64 * 1024 * 1024;

/// Plain source text stays exact; document decoding is a projection, not Narrative approval.
pub fn extract_document_text(path: &Path) -> Result<String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| Error::Contract("document extension is missing or not UTF-8".into()))?
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "md" | "txt" | "pdf" | "docx") {
        return Err(Error::Contract(format!(
            "unsupported text document type: {}",
            path.display()
        )));
    }
    let entry = fs::symlink_metadata(path)?;
    if entry.file_type().is_symlink() || !entry.is_file() {
        return Err(Error::Contract(
            "document must be a regular non-symlink file".into(),
        ));
    }
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_DOCUMENT_BYTES {
        return Err(Error::Contract(
            "document exceeds its regular-file byte limit".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(Error::Contract(
            "document size changed during its bounded read".into(),
        ));
    }
    let text = match extension.as_str() {
        "md" | "txt" => String::from_utf8(bytes)
            .map_err(|error| Error::Contract(format!("document is not UTF-8: {error}")))?,
        "pdf" => pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|error| Error::Contract(format!("PDF text extraction failed: {error}")))?,
        "docx" => docx::extract(&bytes)?,
        _ => unreachable!("extension admitted above"),
    };
    if text.len() > MAX_EXTRACTED_BYTES {
        return Err(Error::Contract(
            "extracted document text exceeds its byte limit".into(),
        ));
    }
    if text.trim().is_empty() {
        return Err(Error::Contract(
            "document contains no extractable text".into(),
        ));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn extracts_docx_text_and_preserves_xml_entities() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sample.docx");
        let file = File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>A &amp; B &#x2014; C</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
        zip.finish().unwrap();

        let text = extract_document_text(&path).unwrap();
        assert!(text.contains("A & B — C"));
    }
}
