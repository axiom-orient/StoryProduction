use promptgen_core::catalog::{CatalogError, CatalogFamily, catalog, catalog_json};
use promptgen_core::image::available_lut_presets;
use promptgen_core::{compilation_schema, image_schema, interview_schema};

use super::super::error::{CliError, EXIT_OK};
use super::super::options::{CatalogOptions, OutputFormat, OutputOptions, SchemaOptions};
use super::super::output::write_output;

pub(crate) fn emit_schema(options: SchemaOptions) -> Result<u8, CliError> {
    let schema = match options.target.as_str() {
        "image" => image_schema(),
        "interview" => interview_schema(),
        "compilation" | "compilation-outcome" => compilation_schema(),
        target => {
            return Err(CliError::usage(format!(
                "unknown schema target {target:?}; expected image, interview, or compilation"
            )));
        }
    };
    write_output(
        &schema.to_pretty_string(),
        options.output.result.as_deref(),
        options.output.force,
    )?;
    Ok(EXIT_OK)
}

pub(crate) fn emit_catalog(options: CatalogOptions) -> Result<u8, CliError> {
    let text = match options.format {
        OutputFormat::Json => catalog_json(options.family)
            .map_err(|error| CliError::validation(format!("catalog load failed: {error}")))?
            .to_pretty_string(),
        // The text listing names four fields, so it reads the typed entries instead
        // of serializing the whole public catalog only to pick strings back out.
        OutputFormat::Text => catalog_text(options.family)
            .map_err(|error| CliError::validation(format!("catalog load failed: {error}")))?,
    };
    write_output(
        &text,
        options.output.result.as_deref(),
        options.output.force,
    )?;
    Ok(EXIT_OK)
}

pub(crate) fn emit_lut_presets(options: OutputOptions) -> Result<u8, CliError> {
    write_output(
        &available_lut_presets().to_pretty_string(),
        options.result.as_deref(),
        options.force,
    )?;
    Ok(EXIT_OK)
}

fn catalog_text(family: Option<CatalogFamily>) -> Result<String, CatalogError> {
    let mut output = String::new();
    for entry in catalog()?
        .entries
        .iter()
        .filter(|entry| family.is_none_or(|family| entry.family == family))
    {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            entry.id,
            entry.family.as_str(),
            entry.name_ko,
            entry.intent
        ));
    }
    Ok(output)
}
