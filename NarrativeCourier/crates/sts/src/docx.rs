//! DOCX formatting and staged publication of supplied screenplay text.

mod assets;

use crate::screenplay_text::{parse, Token};
use assets::{CONTENT_TYPES, DOCUMENT_RELS, ROOT_RELS, STYLES};
use narrative_courier_runtime::store::atomic_publish_file;
use narrative_courier_runtime::{Error, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn export_screenplay_docx(markdown: &str, output: &Path) -> Result<()> {
    let document = document_xml(markdown)?;
    crate::engine::ensure_atomic_file_target(output, "DOCX export")?;
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Contract("system clock is before the Unix epoch".into()))?
        .as_nanos();
    let staged = parent.join(format!(".docx-{}-{stamp}.tmp", std::process::id()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)?;
    let publication = (|| -> Result<()> {
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (path, text) in [
            ("[Content_Types].xml", CONTENT_TYPES),
            ("_rels/.rels", ROOT_RELS),
            ("word/_rels/document.xml.rels", DOCUMENT_RELS),
            ("word/styles.xml", STYLES),
            ("word/document.xml", document.as_str()),
        ] {
            zip.start_file(path, options)
                .map_err(|error| Error::Contract(format!("DOCX entry {path}: {error}")))?;
            zip.write_all(text.as_bytes())?;
        }
        zip.finish()
            .map_err(|error| Error::Contract(format!("DOCX finalize failed: {error}")))?
            .sync_all()?;
        crate::engine::ensure_atomic_file_target(output, "DOCX export")?;
        atomic_publish_file(&staged, output)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    match publication {
        Ok(()) => Ok(()),
        Err(error) => match fs::remove_file(&staged) {
            Ok(()) => Err(error),
            Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => Err(error),
            Err(cleanup) => Err(Error::Contract(format!(
                "{error}; DOCX staging cleanup failed: {cleanup}"
            ))),
        },
    }
}

fn document_xml(markdown: &str) -> Result<String> {
    let mut document = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>");
    for token in parse(markdown) {
        let (style, text) = match token {
            Token::Heading(text) => ("Title", text),
            Token::Scene(text) => ("SceneHeading", text),
            Token::Action(text) => ("Action", text),
            Token::Character(text) => ("CharacterName", text),
            Token::Dialogue(text) => ("Dialogue", text),
            Token::Blank => {
                document.push_str("<w:p/>");
                continue;
            }
        };
        document.push_str("<w:p><w:pPr><w:pStyle w:val=\"");
        document.push_str(style);
        document.push_str("\"/></w:pPr><w:r><w:t xml:space=\"preserve\">");
        append_xml_text(&mut document, text)?;
        document.push_str("</w:t></w:r></w:p>");
    }
    document.push_str("<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1417\" w:right=\"1417\" w:bottom=\"1417\" w:left=\"1701\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>");
    Ok(document)
}

fn append_xml_text(output: &mut String, text: &str) -> Result<()> {
    for character in text.chars() {
        if !(matches!(character, '\t' | '\n' | '\r')
            || matches!(character as u32, 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF))
        {
            return Err(Error::Contract(
                "screenplay contains a character unsupported by XML 1.0".into(),
            ));
        }
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
    Ok(())
}
