use clap::Parser;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
usage: awake [<duration>]

Description

    Stay awake, optionally for the specified duration.

Options

    -h, --help       Print help.
    -v, --version    Print version.
";

#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(short, long)]
    help: bool,

    #[arg(short, long)]
    version: bool,

    #[arg()]
    duration: Option<String>,
}

fn main() {
    match run() {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    let args = Cli::try_parse().map_err(|e| format!("{}\n{HELP}", e.kind()))?;

    if args.help {
        println!("{HELP}");
        return Ok(());
    }

    if args.version {
        println!("awake {VERSION}");
        return Ok(());
    }

    let duration = match args.duration {
        Some(_) => 0,
        None => 1,
    };

    println!("{duration}");

    Ok(())
}
