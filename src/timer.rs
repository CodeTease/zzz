use crate::cli::{Args, Theme};
use crate::process::is_process_running;
use crate::theme::{create_progress_bar, get_status_frame};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use signal_hook::consts::signal::{SIGCONT, SIGINT, SIGTERM, SIGTSTP};
use signal_hook::iterator::Signals;
use std::process::Command;
use std::time::{Duration, Instant};

pub struct RawModeGuard {
    pub active: bool,
}

impl RawModeGuard {
    pub fn new() -> Self {
        let active = terminal::enable_raw_mode().is_ok();
        Self { active }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = terminal::disable_raw_mode();
        }
    }
}

// Helper function to format time nicely (MM:SS or HH:MM:SS)
fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TimerOutcome {
    Completed,
    Interrupted,
    WatchedProcessTerminated,
}

pub fn run_sleep_timer(
    mut duration: Duration,
    quiet: bool,
    raw: bool,
    theme: &Theme,
    watch_pid: Option<u32>,
    completion_message: &str,
    step: Duration,
) -> Result<TimerOutcome, Box<dyn std::error::Error>> {
    if duration.as_millis() == 0 {
        if raw && !quiet {
            println!("{:10}", format_duration(Duration::ZERO));
        }
        return Ok(TimerOutcome::Completed);
    }

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGTSTP, SIGCONT])?;

    let pb = if !quiet && !raw {
        let total_millis: u64 = duration.as_millis().try_into().unwrap_or(u64::MAX);
        Some(create_progress_bar(total_millis, theme)?)
    } else {
        None
    };

    let raw_mode_guard = RawModeGuard::new();

    let mut start = Instant::now();
    let mut paused_accum = Duration::ZERO;
    let mut pause_start: Option<Instant> = None;

    loop {
        let effective_elapsed = if let Some(p_start) = pause_start {
            (p_start - start).saturating_sub(paused_accum)
        } else {
            start.elapsed().saturating_sub(paused_accum)
        };

        if effective_elapsed >= duration {
            break;
        }

        let remaining = duration.saturating_sub(effective_elapsed);

        // Check watched PID
        if let Some(pid) = watch_pid {
            if !is_process_running(pid) {
                if let Some(ref pb) = pb {
                    pb.finish_with_message(format!("Watched process PID {} terminated. Exiting!", pid));
                } else if raw && !quiet {
                    println!("\nWatched process PID {} terminated. Exiting!", pid);
                }
                return Ok(TimerOutcome::WatchedProcessTerminated);
            }
        }

        // Handle unix signals
        for sig in signals.pending() {
            match sig {
                SIGINT | SIGTERM => {
                    if let Some(ref pb) = pb {
                        pb.abandon_with_message("Cancelled! 🛑");
                    } else if raw && !quiet {
                        eprintln!("\nCancelled! 🛑");
                    }
                    return Ok(TimerOutcome::Interrupted);
                }
                SIGTSTP => {
                    if pause_start.is_none() {
                        pause_start = Some(Instant::now());
                    }
                }
                SIGCONT => {
                    if let Some(p_start) = pause_start {
                        paused_accum += p_start.elapsed();
                        pause_start = None;
                    }
                }
                _ => {}
            }
        }

        // Update UI
        if let Some(ref pb) = pb {
            let elapsed_millis: u64 = effective_elapsed.as_millis().try_into().unwrap_or(u64::MAX);
            let ratio = if duration.as_secs_f64() > 0.0 {
                effective_elapsed.as_secs_f64() / duration.as_secs_f64()
            } else {
                1.0
            };

            let time_str = format_duration(effective_elapsed);
            let total_str = format_duration(duration);
            let status = get_status_frame(ratio);

            if pause_start.is_some() {
                pb.set_message(format!("⏸️ [PAUSED at {}/{}] Press Space/P to resume", time_str, total_str));
            } else {
                pb.set_message(format!("⏳ [{}/{}] {}", time_str, total_str, status));
            }
            pb.set_position(elapsed_millis);
        } else if raw && !quiet {
            let time_str = format_duration(remaining);
            print!("\r{:10}", time_str);
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }

        let poll_timeout = std::cmp::min(Duration::from_millis(100), remaining);

        let mut key_event_occurred = false;
        if raw_mode_guard.active {
            if event::poll(poll_timeout)? {
                key_event_occurred = true;
                while let Ok(true) = event::poll(Duration::from_millis(0)) {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        if key_event.code == KeyCode::Char('c')
                            && key_event.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if let Some(ref pb) = pb {
                                pb.abandon_with_message("Cancelled! 🛑");
                            } else if raw && !quiet {
                                eprintln!("\nCancelled! 🛑");
                            }
                            return Ok(TimerOutcome::Interrupted);
                        }

                        match key_event.code {
                            KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') => {
                                if let Some(p_start) = pause_start {
                                    paused_accum += p_start.elapsed();
                                    pause_start = None;
                                } else {
                                    pause_start = Some(Instant::now());
                                }
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                duration = duration.saturating_add(step);
                                if let Some(ref pb) = pb {
                                    pb.set_length(duration.as_millis().try_into().unwrap_or(u64::MAX));
                                }
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                duration = duration.saturating_sub(step);
                                if let Some(ref pb) = pb {
                                    pb.set_length(duration.as_millis().try_into().unwrap_or(u64::MAX));
                                }
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                if let Some(ref pb) = pb {
                                    pb.finish_with_message("Skipped!");
                                } else if raw && !quiet {
                                    println!("\r{:10}", format_duration(Duration::ZERO));
                                }
                                return Ok(TimerOutcome::Completed);
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                start = Instant::now();
                                paused_accum = Duration::ZERO;
                                pause_start = None;
                                if let Some(ref pb) = pb {
                                    pb.set_position(0);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if !key_event_occurred && !raw_mode_guard.active {
            std::thread::sleep(poll_timeout);
        }
    }

    if let Some(pb) = pb {
        let total_millis: u64 = duration.as_millis().try_into().unwrap_or(u64::MAX);
        pb.set_position(total_millis);
        pb.finish_with_message(completion_message.to_string());
    } else if raw && !quiet {
        println!("\r{:10}", format_duration(Duration::ZERO));
    }

    Ok(TimerOutcome::Completed)
}

pub fn run_pomo_mode(args: &Args) -> Result<TimerOutcome, Box<dyn std::error::Error>> {
    let mut cycle = 1;
    loop {
        let work_msg = format!("🍅 Pomodoro Cycle {}: Work finished!", cycle);
        if !args.raw && !args.quiet {
            println!("🍅 Starting Pomodoro Cycle {} Work phase ({:?})...", cycle, args.pomo_work);
        }
        let outcome = run_sleep_timer(
            args.pomo_work,
            args.quiet,
            args.raw,
            &args.theme,
            args.watch,
            &work_msg,
            args.step,
        )?;
        if outcome != TimerOutcome::Completed {
            return Ok(outcome);
        }

        let break_msg = format!("☕ Pomodoro Cycle {}: Break finished!", cycle);
        if !args.raw && !args.quiet {
            println!("☕ Starting Pomodoro Cycle {} Break phase ({:?})...", cycle, args.pomo_break);
        }
        let outcome = run_sleep_timer(
            args.pomo_break,
            args.quiet,
            args.raw,
            &args.theme,
            args.watch,
            &break_msg,
            args.step,
        )?;
        if outcome != TimerOutcome::Completed {
            return Ok(outcome);
        }

        cycle += 1;
    }
}

pub fn execute_command(cmd_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Executing command: {}", cmd_str);
    #[cfg(unix)]
    let status = Command::new("sh").arg("-c").arg(cmd_str).status()?;

    #[cfg(not(unix))]
    let status = Command::new("cmd").arg("/C").arg(cmd_str).status()?;

    if !status.success() {
        eprintln!("Command exited with status: {}", status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_sleep_timer_short() {
        let res = run_sleep_timer(
            Duration::from_millis(100),
            true,
            false,
            &Theme::Classic,
            None,
            "Done",
            Duration::from_secs(30),
        );
        assert_eq!(res.unwrap(), TimerOutcome::Completed);
    }

    #[test]
    fn test_run_sleep_timer_watched_dead_pid() {
        let res = run_sleep_timer(
            Duration::from_secs(10),
            true,
            false,
            &Theme::Classic,
            Some(999999),
            "Done",
            Duration::from_secs(30),
        );
        assert_eq!(res.unwrap(), TimerOutcome::WatchedProcessTerminated);
    }
}

