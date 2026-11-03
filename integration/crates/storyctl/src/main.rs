fn main() {
    storyctl::effects::process::install_signal_handlers();
    std::process::exit(storyctl::cli::main(std::env::args().skip(1).collect()));
}
