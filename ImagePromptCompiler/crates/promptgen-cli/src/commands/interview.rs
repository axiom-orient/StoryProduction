use promptgen_core::{InterviewRequest, InterviewStatus, run_interview};

use super::super::error::{CliError, EXIT_OK, EXIT_VALIDATION};
use super::super::options::{CompileOptions, OutputFormat};
use super::super::output::{read_json, write_output};

pub(crate) fn run_guided_interview(options: CompileOptions) -> Result<u8, CliError> {
    let request = InterviewRequest::from_json(read_json(&options.input)?)
        .map_err(|error| CliError::validation(error.to_string()))?;
    let outcome = run_interview(&request);
    let text = match options.format {
        OutputFormat::Json => outcome.to_json().to_pretty_string(),
        OutputFormat::Text => interview_to_text(&outcome),
    };
    write_output(
        &text,
        options.output.result.as_deref(),
        options.output.force,
    )?;
    Ok(if outcome.status == InterviewStatus::Invalid {
        EXIT_VALIDATION
    } else {
        EXIT_OK
    })
}

fn interview_to_text(outcome: &promptgen_core::InterviewOutcome) -> String {
    if let Some(compilation) = &outcome.compilation {
        return compilation.to_text();
    }
    let mut text = format!(
        "status={}\nkind={}\n",
        outcome.status.as_str(),
        outcome.kind.as_str()
    );
    if !outcome.inferences.is_empty() {
        text.push_str("\n[inferences]\n");
        for inference in &outcome.inferences {
            text.push_str("- ");
            text.push_str(inference);
            text.push('\n');
        }
    }
    if !outcome.questions.is_empty() {
        text.push_str("\n[questions]\n");
        for question in &outcome.questions {
            text.push_str(&format!(
                "- {}: {} ({})\n  {}\n",
                question.id, question.label, question.control, question.help
            ));
        }
    }
    text
}
