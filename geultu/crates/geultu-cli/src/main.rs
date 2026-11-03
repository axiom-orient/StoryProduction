mod cli;
mod mcp;

use serde_json::json;

fn main() {
    match cli::commands::run() {
        Ok(()) => {}
        Err(cli::CliError::Usage(message)) => {
            eprintln!("{message}\n\n{}", cli::usage());
            std::process::exit(2);
        }
        Err(cli::CliError::Engine(error)) => {
            let body = error.body();
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json!({"ok":false,"error":body}))
                    .unwrap_or_else(|_| error.to_string())
            );
            std::process::exit(1);
        }
        Err(cli::CliError::Other(message)) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}
