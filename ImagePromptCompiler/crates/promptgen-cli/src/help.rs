use promptgen_mcp::{DEFAULT_MCP_PORT, MAX_BODY_BYTES};

pub(super) fn print_help() {
    println!(
        "promptgen {}\n\nUSAGE:\n  promptgen <COMMAND> [OPTIONS]\n\nCOMMANDS:\n  image         compile an image prompt or execute it with Codex image generation\n  interview     ask only for missing image contract fields and compile when complete\n  serve         run the loopback-only image Studio\n  mcp           expose the stateless image MCP compiler over stdio or Streamable HTTP\n  schema        print image/interview/compilation JSON Schema\n  catalog       list the six representative image outcomes\n  lut-presets   list resolved photographic LUT profiles\n  version       print the version\n\nRun `promptgen <COMMAND> --help` for command options.",
        env!("CARGO_PKG_VERSION")
    );
}

pub(super) fn print_interview_help() {
    println!(
        "USAGE: promptgen interview [--input FILE|-] [--format json|text] [--result FILE] [--force]"
    );
}

pub(super) fn print_image_help() {
    println!(
        "USAGE: promptgen image [--input FILE|-] [--mode prompt-only|codex-imagegen|openai-image-edit|openai-image-generate] [--format json|text] [--result FILE] [--image-output FILE] [--reference-image FILE] [--codex-binary FILE] [--codex-home DIR] [--timeout-seconds 1..3600] [--artifact-wait-seconds 1..600] [--force]"
    );
}

pub(super) fn print_catalog_help() {
    println!(
        "USAGE: promptgen catalog [--family all|category|promo|typography|look|movement|response|type-art|editorial] [--format json|text] [--result FILE] [--force]"
    );
}

pub(super) fn print_schema_help() {
    println!("USAGE: promptgen schema <image|interview|compilation> [--result FILE] [--force]");
}

pub(super) fn print_serve_help() {
    println!(
        "USAGE: promptgen serve [--bind 127.0.0.1:4173] [--output-dir DIR] [--catalog-dir DIR] [--codex-binary FILE] [--codex-home DIR] [--timeout-seconds 1..3600] [--artifact-wait-seconds 1..600]"
    );
}

pub(super) fn print_mcp_help() {
    println!(
        "USAGE: promptgen mcp [--transport stdio|http] [--bind 127.0.0.1:{mcp_port}] [--bearer-token TOKEN] [--allowed-host HOST[,HOST...]] [--max-body-bytes 1..={max_body_bytes}]\n\nDefault transport is stdio. HTTP is stateless-only and advertises MCP protocol 2026-07-28. Prefer PROMPTGEN_MCP_BEARER_TOKEN over the command-line token for remote binds.",
        mcp_port = DEFAULT_MCP_PORT,
        max_body_bytes = MAX_BODY_BYTES,
    );
}

pub(super) fn print_lut_help() {
    println!("USAGE: promptgen lut-presets [--result FILE] [--force]");
}
