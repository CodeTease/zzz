mod cli;
mod process;
mod theme;
mod time;
mod timer;

use clap::Parser;
use cli::Args;
use time::parse_until_target;
use timer::{execute_command, run_pomo_mode, run_sleep_timer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.pomo {
        run_pomo_mode(&args)?;
        if let Some(ref cmd) = args.then {
            execute_command(cmd)?;
        }
        return Ok(());
    }

    let duration = if let Some(d) = args.duration {
        d
    } else if let Some(ref until_str) = args.until {
        parse_until_target(until_str)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
    } else {
        eprintln!("Error: Please specify a duration (e.g. `zzz 10s`), `--until <TIME>`, or `--pomo`.");
        std::process::exit(1);
    };

    let completed = run_sleep_timer(
        duration,
        args.quiet,
        &args.theme,
        args.watch,
        &args.message,
    )?;

    if completed {
        if let Some(ref cmd) = args.then {
            execute_command(cmd)?;
        }
    }

    Ok(())
}
