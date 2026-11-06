mod animatic;
mod browser;
mod gif_preview;
mod io;
mod process;
mod producer;
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf, sync::atomic::Ordering};
extern "C" fn interrupt(_signal: i32) {
    io::INTERRUPTED.store(true, Ordering::SeqCst);
}
fn run() -> io::Result<serde_json::Value> {
    let mut args = std::env::args().skip(1);
    let operation = args.next().ok_or("operation required")?;
    if operation == "--help" {
        println!("smc-animatic render|status|cancel|recover --project ROOT --attempt ID [--packet REL --assets REL --asset-review REL --motion full|reduced --owner-root PATH]");
        std::process::exit(0);
    }
    let mut options = BTreeMap::new();
    while let Some(flag) = args.next() {
        io::check(flag.starts_with("--"), "UNKNOWN_OPTION")?;
        let value = args.next().ok_or("MISSING_OPTION_VALUE")?;
        io::check(options.insert(flag, value).is_none(), "DUPLICATE_OPTION")?;
    }
    let root = PathBuf::from(options.remove("--project").ok_or("--project required")?);
    let owner = options.remove("--owner-root").map(PathBuf::from);
    let context = animatic::Context::new(&root, owner.as_deref())?;
    if ["status", "cancel", "recover"].contains(&operation.as_str()) {
        let attempt = options.remove("--attempt").ok_or("--attempt required")?;
        io::check(options.is_empty(), "UNKNOWN_OPTION")?;
        return match operation.as_str() {
            "status" => context.status(&attempt),
            "cancel" => context.cancel(&attempt),
            _ => context.recover(&attempt),
        };
    }
    io::check(
        ["render", "inspect"].contains(&operation.as_str()),
        "UNKNOWN_OPERATION",
    )?;
    let attempt = if operation == "inspect" {
        options
            .remove("--attempt")
            .unwrap_or_else(|| "inspect".into())
    } else {
        options.remove("--attempt").ok_or("--attempt required")?
    };
    let packet = options.remove("--packet").ok_or("--packet required")?;
    let assets = options.remove("--assets").ok_or("--assets required")?;
    let review = options
        .remove("--asset-review")
        .ok_or("--asset-review required")?;
    let motion = options.remove("--motion").unwrap_or_else(|| "full".into());
    io::check(
        ["full", "reduced"].contains(&motion.as_str()) && options.is_empty(),
        "UNKNOWN_OPTION_OR_MOTION",
    )?;
    let input = animatic::Input {
        packet,
        assets,
        review,
        attempt,
        reduced: motion == "reduced",
    };
    if operation == "inspect" {
        context.inspect(&input)
    } else {
        context.render(&input)
    }
}
fn main() {
    unsafe {
        libc::signal(libc::SIGINT, interrupt as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, interrupt as *const () as libc::sighandler_t);
    }
    match run() {
        Ok(value) => {
            println!("{value}");
            if !matches!(
                value["status"].as_str(),
                Some("candidate" | "validated" | "running")
            ) {
                std::process::exit(3);
            }
        }
        Err(error) => {
            let status = if error.starts_with("OUTCOME_UNKNOWN:") {
                "outcome_unknown"
            } else if error.contains("_UNAVAILABLE")
                || error.contains("_NOT_EXECUTABLE")
                || error == "ATTEMPT_NOT_STARTED"
            {
                "not_started"
            } else {
                "not_applied"
            };
            let mut value = json!({"status":status,"detail":error});
            let args: Vec<_> = std::env::args().collect();
            if let Some(attempt) = args.windows(2).find(|pair| pair[0] == "--attempt") {
                value["attemptId"] = attempt[1].clone().into();
            }
            if status == "outcome_unknown" || status == "not_started" {
                println!("{value}");
            }
            eprintln!("{value}");
            std::process::exit(if status == "outcome_unknown" { 3 } else { 2 });
        }
    }
}
