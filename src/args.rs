use std::path::PathBuf;

pub struct Args {
    pub config: Option<PathBuf>,
}

impl Args {
    pub fn parse() -> Self {
        let mut config = None;
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-c" | "--config" => {
                    config = args.next().map(PathBuf::from);
                }
                "-h" | "--help" => {
                    println!("kickoff - A fast and minimal program launcher for macOS");
                    println!();
                    println!("USAGE:");
                    println!("    kickoff [OPTIONS]");
                    println!();
                    println!("OPTIONS:");
                    println!("    -c, --config <PATH>    Path to config file");
                    println!("    -h, --help             Print help information");
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("Unknown argument: {}", arg);
                    eprintln!("Try 'kickoff --help' for more information");
                    std::process::exit(1);
                }
            }
        }

        Self { config }
    }
}
