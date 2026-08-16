mod cli;
mod error;
mod process;
mod theme;
mod time;
mod timer;

use clap::Parser;
use cli::Args;
use error::ZzzError;
use time::parse_until_target;
use timer::{TimerOutcome, execute_command, run_pomo_mode, run_sleep_timer};

#[cfg(unix)]
fn is_background() -> bool {
    unsafe { libc::tcgetpgrp(libc::STDIN_FILENO) != libc::getpgrp() }
}

#[cfg(windows)]
fn is_background() -> bool {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetConsoleWindow, GetStdHandle, STD_INPUT_HANDLE,
    };

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return true;
        }
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return true;
        }
        if GetConsoleWindow().is_null() {
            return true;
        }
        false
    }
}

#[cfg(not(any(unix, windows)))]
fn is_background() -> bool {
    false
}

fn main() -> Result<(), ZzzError> {
    let args = Args::parse();

    if is_background() && !args.no_interaction && !args.quiet {
        eprintln!("Error: `zzz` TUI mode requires a foreground terminal.");
        eprintln!("Use `--no-interaction` or `--quiet` to run in background.");
        std::process::exit(1);
    }

    let outcome = if args.pomo {
        run_pomo_mode(&args)?
    } else {
        let duration = if let Some(d) = args.duration {
            d
        } else if let Some(ref until_str) = args.until {
            parse_until_target(until_str)?
        } else {
            eprintln!(
                "Error: Please specify a duration (e.g. `zzz 10s`), `--until <TIME>`, or `--pomo`."
            );
            std::process::exit(1);
        };

        run_sleep_timer(
            duration,
            args.quiet,
            args.raw,
            args.no_interaction,
            &args.theme,
            args.watch,
            &args.message,
            args.step,
        )?
    };

    match outcome {
        TimerOutcome::Completed => {
            if let Some(ref cmd) = args.then {
                execute_command(cmd)?;
            }
            std::process::exit(0);
        }
        TimerOutcome::Interrupted => {
            std::process::exit(130);
        }
        TimerOutcome::WatchedProcessTerminated => {
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_background_callable() {
        let _ = is_background();
    }
}
