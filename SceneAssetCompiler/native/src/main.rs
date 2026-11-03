use serde_json::json;
fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("sac-animatic <submit|inspect|review> --project-root ROOT --packet REL [--draft REL|--pack REL] [--output REL] [--expected-hash HASH --reviewer ID --decision approved|rejected --note TEXT] [--owner-root ROOT]");
        return;
    }
    match sac_animatic::parse_options(&arguments).and_then(|options| sac_animatic::run(&options)) {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(error) => {
            let value = json!({"status":if error.unknown{"outcome_unknown"}else{"not_applied"},"error":error.message});
            if error.unknown {
                println!("{value}");
                std::process::exit(3);
            } else {
                eprintln!("{value}");
                std::process::exit(2);
            }
        }
    }
}
