//! narrative-courier-mcp binary — MCP stdio server entry point.
//!
//! Human authority notice: this surface never exposes STS gate decisions or
//! delivery review. See crate docs and README "Authority".

fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(e) = runtime.block_on(narrative_courier_mcp::serve_stdio()) {
        eprintln!("narrative-courier-mcp error: {e}");
        std::process::exit(1);
    }
}
