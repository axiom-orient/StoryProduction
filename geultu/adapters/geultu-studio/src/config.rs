use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

const DEFAULT_LISTEN: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8765);

pub(super) struct Config {
    pub(super) workspace: PathBuf,
    pub(super) listen: SocketAddr,
    pub(super) writer_program: Option<PathBuf>,
    pub(super) writer_args: Vec<String>,
}
impl Config {
    pub(super) fn parse() -> Result<Self, String> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Self::from_args(&args)
    }

    pub(super) fn from_args(args: &[String]) -> Result<Self, String> {
        let mut workspace = PathBuf::from(".");
        let mut listen = DEFAULT_LISTEN;
        let mut writer_program = None;
        let mut writer_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--workspace" => {
                    index += 1;
                    workspace =
                        PathBuf::from(args.get(index).ok_or("--workspace requires a path")?);
                }
                "--listen" => {
                    index += 1;
                    listen = args
                        .get(index)
                        .ok_or("--listen requires IP:PORT")?
                        .parse()
                        .map_err(|_| "invalid --listen value")?;
                }
                "--writer-program" => {
                    index += 1;
                    writer_program = Some(PathBuf::from(
                        args.get(index).ok_or("--writer-program requires a path")?,
                    ));
                }
                "--writer-arg" => {
                    index += 1;
                    writer_args.push(
                        args.get(index)
                            .ok_or("--writer-arg requires a value")?
                            .clone(),
                    );
                }
                "--help" | "-h" => {
                    println!("GeultuStudio [--workspace PATH] [--listen {DEFAULT_LISTEN}] [--writer-program PATH --writer-arg ARG ...]");
                    std::process::exit(0);
                }
                value => return Err(format!("unknown argument '{value}'")),
            }
            index += 1;
        }
        Ok(Self {
            workspace,
            listen,
            writer_program,
            writer_args,
        })
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if !self.listen.ip().is_loopback() {
            return Err(
                "non-loopback Studio listeners are disabled; use a loopback address".into(),
            );
        }
        if self.workspace.as_os_str().is_empty() {
            return Err("workspace path must not be empty".into());
        }
        if self.writer_program.is_none() && !self.writer_args.is_empty() {
            return Err("--writer-arg requires --writer-program".into());
        }
        Ok(())
    }
}
