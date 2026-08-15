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

pub fn run_sleep_timer(
    duration: Duration,
    quiet: bool,
    theme: &Theme,
    watch_pid: Option<u32>,
    completion_message: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let total_millis: u64 = duration.as_millis().try_into().unwrap_or(u64::MAX);

    if total_millis == 0 {
        return Ok(true);
    }

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGTSTP, SIGCONT])?;

    let pb = if !quiet {
        Some(create_progress_bar(total_millis, theme)?)
    } else {
        None
    };

    let raw_mode_guard = RawModeGuard::new();

    let start = Instant::now();
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

        // Handle unix signals
        for sig in signals.pending() {
            match sig {
                SIGINT | SIGTERM => {
                    if let Some(ref pb) = pb {
                        pb.abandon_with_message("Cancelled! 🛑");
                    }
                    return Ok(false);
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

        // Handle crossterm keypresses if raw mode is active
        if raw_mode_guard.active {
            while let Ok(true) = event::poll(Duration::from_millis(0)) {
                if let Ok(Event::Key(key_event)) = event::read() {
                    if key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        if let Some(ref pb) = pb {
                            pb.abandon_with_message("Cancelled! 🛑");
                        }
                        return Ok(false);
                    }

                    if key_event.code == KeyCode::Char(' ') || key_event.code == KeyCode::Char('p') || key_event.code == KeyCode::Char('P') {
                        if let Some(p_start) = pause_start {
                            paused_accum += p_start.elapsed();
                            pause_start = None;
                        } else {
                            pause_start = Some(Instant::now());
                        }
                    }
                }
            }
        }

        // Check watched PID
        if let Some(pid) = watch_pid {
            if !is_process_running(pid) {
                if let Some(ref pb) = pb {
                    pb.finish_with_message(format!("Watched process PID {} terminated. Exiting!", pid));
                }
                return Ok(true);
            }
        }

        // Update UI
        if let Some(ref pb) = pb {
            let elapsed_millis: u64 = effective_elapsed.as_millis().try_into().unwrap_or(u64::MAX);
            let ratio = effective_elapsed.as_secs_f64() / duration.as_secs_f64();

            let time_str = format_duration(effective_elapsed);
            let total_str = format_duration(duration);

            if pause_start.is_some() {
                // UI frozen, the time has clearly stopped
                pb.set_message(format!("⏸️ [PAUSED at {}/{}] Press Space/P to resume", time_str, total_str));

                // Hacky: Reset the position so the bar stays in place
                let elapsed_millis: u64 = effective_elapsed.as_millis().try_into().unwrap_or(u64::MAX);
                pb.set_position(elapsed_millis);
            } else {
                pb.set_message(get_status_frame(ratio));
            }
            pb.set_position(elapsed_millis);

            // Sleep briefly while waiting for an event/signal to resume, avoid wasting CPU cycles
            std::thread::sleep(Duration::from_millis(50));
        }

        let remaining = duration.saturating_sub(effective_elapsed);
        std::thread::sleep(std::cmp::min(Duration::from_millis(50), remaining));
    }

    if let Some(pb) = pb {
        pb.set_position(total_millis);
        pb.finish_with_message(completion_message.to_string());
    }

    Ok(true)
}

pub fn run_pomo_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut cycle = 1;
    loop {
        let work_msg = format!("🍅 Pomodoro Cycle {}: Work finished!", cycle);
        println!("🍅 Starting Pomodoro Cycle {} Work phase ({:?})...", cycle, args.pomo_work);
        let completed = run_sleep_timer(args.pomo_work, args.quiet, &args.theme, args.watch, &work_msg)?;
        if !completed {
            break;
        }

        let break_msg = format!("☕ Pomodoro Cycle {}: Break finished!", cycle);
        println!("☕ Starting Pomodoro Cycle {} Break phase ({:?})...", cycle, args.pomo_break);
        let completed = run_sleep_timer(args.pomo_break, args.quiet, &args.theme, args.watch, &break_msg)?;
        if !completed {
            break;
        }

        cycle += 1;
    }
    Ok(())
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
            &Theme::Classic,
            None,
            "Done",
        );
        assert!(res.is_ok());
        assert!(res.unwrap());
    }

    #[test]
    fn test_run_sleep_timer_watched_dead_pid() {
        let res = run_sleep_timer(
            Duration::from_secs(10),
            true,
            &Theme::Classic,
            Some(999999),
            "Done",
        );
        assert!(res.is_ok());
        assert!(res.unwrap());
    }
}
