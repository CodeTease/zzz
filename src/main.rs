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
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use timer::{TimerOutcome, execute_command, run_pomo_mode, run_sleep_timer, run_stopwatch_timer};

pub fn get_lock_command() -> &'static str {
    if cfg!(target_os = "macos") {
        "pmset displaysleepnow"
    } else if cfg!(windows) {
        "rundll32.exe user32.dll,LockWorkStation"
    } else {
        "xdg-screensaver lock"
    }
}

pub fn get_shutdown_command() -> &'static str {
    if cfg!(target_os = "macos") {
        "shutdown -h now"
    } else if cfg!(windows) {
        "shutdown /s /t 0"
    } else {
        "shutdown -h now"
    }
}

fn render_menu(selected: usize) {
    let options = [
        "Lock screen",
        "Shutdown",
        "Do nothing",
        "Run custom command",
    ];

    print!("Post-timer action menu:\r\n");
    for (i, opt) in options.iter().enumerate() {
        if i == selected {
            print!(" > [{}] {}\r\n", i + 1, opt);
        } else {
            print!("   [{}] {}\r\n", i + 1, opt);
        }
    }
    print!("(Use Up/Down or 1-4, Enter to select)\r\n");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn clear_menu() {
    use crossterm::cursor::MoveUp;
    use crossterm::terminal::{Clear, ClearType};
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, MoveUp(6));
    for _ in 0..6 {
        let _ = crossterm::execute!(stdout, Clear(ClearType::CurrentLine));
        print!("\r\n");
    }
    let _ = crossterm::execute!(stdout, MoveUp(6));
    let _ = stdout.flush();
}

pub fn run_then_menu() -> Result<(), ZzzError> {
    if is_background() {
        eprintln!("`--then-menu` requires an interactive terminal.");
        return Ok(());
    }

    let _guard = timer::RawModeGuard::new(false);

    let mut selected: usize = 0;
    render_menu(selected);

    loop {
        if let Ok(Event::Key(key_event)) = event::read() {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    selected = if selected == 0 { 3 } else { selected - 1 };
                    clear_menu();
                    render_menu(selected);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    selected = (selected + 1) % 4;
                    clear_menu();
                    render_menu(selected);
                }
                KeyCode::Char('1') => {
                    selected = 0;
                    break;
                }
                KeyCode::Char('2') => {
                    selected = 1;
                    break;
                }
                KeyCode::Char('3') => {
                    selected = 2;
                    break;
                }
                KeyCode::Char('4') => {
                    selected = 3;
                    break;
                }
                KeyCode::Enter => {
                    break;
                }
                KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    drop(_guard);

    match selected {
        0 => {
            let cmd = get_lock_command();
            execute_command(cmd)?;
        }
        1 => {
            let cmd = get_shutdown_command();
            execute_command(cmd)?;
        }
        2 => {
            println!("Do nothing selected.");
        }
        3 => {
            use std::io::Write;
            print!("Enter custom command: ");
            std::io::stdout().flush()?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                execute_command(trimmed)?;
            }
        }
        _ => {}
    }

    Ok(())
}

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

    let outcome = if args.stopwatch {
        run_stopwatch_timer(&args)?
    } else if args.pomo {
        run_pomo_mode(&args)?
    } else {
        let duration = if let Some(d) = args.duration {
            d
        } else if let Some(ref until_str) = args.until {
            parse_until_target(until_str)?
        } else {
            eprintln!(
                "Error: Please specify a duration (e.g. `zzz 10s`), `--until <TIME>`, `--pomo`, or `--stopwatch`."
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
            args.on_pause.as_deref(),
            args.on_tick.as_deref(),
        )?
    };

    match outcome {
        TimerOutcome::Completed => {
            if args.then_menu {
                run_then_menu()?;
            } else if let Some(ref cmd) = args.then {
                execute_command(cmd)?;
            }
            std::process::exit(0);
        }
        TimerOutcome::Interrupted => {
            if let Some(ref cmd) = args.on_interrupt {
                let _ = execute_command(cmd);
            }
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

    #[test]
    fn test_os_command_helpers() {
        let lock_cmd = get_lock_command();
        let shutdown_cmd = get_shutdown_command();
        assert!(!lock_cmd.is_empty());
        assert!(!shutdown_cmd.is_empty());
    }
}
