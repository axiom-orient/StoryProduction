#![forbid(unsafe_code)]

mod api;
mod execution;
mod files;
mod limits;
mod reference;
mod request;
mod response;
mod support;
mod transport;

pub use api::{OpenAiEditReceipt, OpenAiExecutionError, OpenAiRunConfig};
pub use execution::{execute_image_edit, execute_image_generate};
pub use reference::openai_paths_alias;

const API_ENDPOINT: &str = "https://api.openai.com/v1/images/edits";

#[cfg(test)]
mod tests;
