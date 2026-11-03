//! CLI-compatible asset/project root selection for the stdio server.

use narrative_courier_app_config::resolve_roots as resolve_product_roots;
use narrative_courier_app_config::Roots;
use std::path::PathBuf;

/// Parse MCP-specific flags and delegate common precedence to app-config.
pub fn resolve_roots(args: &[String]) -> Result<Roots, Box<dyn std::error::Error>> {
    let mut assets = None;
    let mut projects = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--assets-root" | "--projects-root" => {
                let slot = if arg == "--assets-root" {
                    &mut assets
                } else {
                    &mut projects
                };
                if slot.is_some() {
                    return Err(format!("duplicate option: {arg}").into());
                }
                let value = iter.next().ok_or_else(|| format!("{arg} requires PATH"))?;
                if value.is_empty() || matches!(value.as_str(), "--assets-root" | "--projects-root")
                {
                    return Err(format!("{arg} requires PATH before another option").into());
                }
                *slot = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    Ok(resolve_product_roots(assets, projects)?)
}
