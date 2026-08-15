use crate::cli::{Args, Theme};
use crate::error::ZzzError;
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
    pub fn new(no_interaction: bool) -> Self {
        let active = if no_interaction {
            false
        } else {
            terminal::enable_raw_mode().is_ok()
        };
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyAction {
    Cancel,
    TogglePause,
    AddStep,
    SubStep,
    Skip,
    Reset,
}

fn handle_key_input(poll_timeout: Duration) -> Result<(bool, Option<KeyAction>), ZzzError> {
    if event::poll(poll_timeout)? {
        let mut last_action = None;
        while let Ok(true) = event::poll(Duration::from_millis(0)) {
            if let Ok(Event::Key(key_event)) = event::read() {
                if key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    return Ok((true, Some(KeyAction::Cancel)));
                }

                let action = match key_event.code {
                    KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') => Some(KeyAction::TogglePause),
                    KeyCode::Char('+') | KeyCode::Char('=') => Some(KeyAction::AddStep),
                    KeyCode::Char('-') | KeyCode::Char('_') => Some(KeyAction::SubStep),
                    KeyCode::Char('s') | KeyCode::Char('S') => Some(KeyAction::Skip),
                    KeyCode::Char('r') | KeyCode::Char('R') => Some(KeyAction::Reset),
                    _ => None,
                };

                if action.is_some() {
                    last_action = action;
                }
            }
        }
        Ok((true, last_action))
    } else {
        Ok((false, None))
    }
}

fn update_ui(
    pb: Option<&indicatif::ProgressBar>,
    raw: bool,
    quiet: bool,
    effective_elapsed: Duration,
    duration: Duration,
    remaining: Duration,
    is_paused: bool,
) {
    if let Some(pb) = pb {
        let elapsed_millis: u64 = effective_elapsed.as_millis().try_into().unwrap_or(u64::MAX);
        let ratio = if duration.as_secs_f64() > 0.0 {
            effective_elapsed.as_secs_f64() / duration.as_secs_f64()
        } else {
            1.0
        };

        let time_str = format_duration(effective_elapsed);
        let total_str = format_duration(duration);
        let status = get_status_frame(ratio);

        if is_paused {
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
}

pub fn run_sleep_timer(
    mut duration: Duration,
    quiet: bool,
    raw: bool,
    no_interaction: bool,
    theme: &Theme,
    watch_pid: Option<u32>,
    completion_message: &str,
    step: Duration,
) -> Result<TimerOutcome, ZzzError> {
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

    let raw_mode_guard = RawModeGuard::new(no_interaction);

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
        update_ui(
            pb.as_ref(),
            raw,
            quiet,
            effective_elapsed,
            duration,
            remaining,
            pause_start.is_some(),
        );

        let poll_timeout = std::cmp::min(Duration::from_millis(100), remaining);

        let mut key_event_occurred = false;
        if raw_mode_guard.active {
            let (occurred, action) = handle_key_input(poll_timeout)?;
            key_event_occurred = occurred;

            if let Some(act) = action {
                match act {
                    KeyAction::Cancel => {
                        if let Some(ref pb) = pb {
                            pb.abandon_with_message("Cancelled! 🛑");
                        } else if raw && !quiet {
                            eprintln!("\nCancelled! 🛑");
                        }
                        return Ok(TimerOutcome::Interrupted);
                    }
                    KeyAction::TogglePause => {
                        if let Some(p_start) = pause_start {
                            paused_accum += p_start.elapsed();
                            pause_start = None;
                        } else {
                            pause_start = Some(Instant::now());
                        }
                    }
                    KeyAction::AddStep => {
                        duration = duration.saturating_add(step);
                        if let Some(ref pb) = pb {
                            pb.set_length(duration.as_millis().try_into().unwrap_or(u64::MAX));
                        }
                    }
                    KeyAction::SubStep => {
                        duration = duration.saturating_sub(step);
                        if let Some(ref pb) = pb {
                            pb.set_length(duration.as_millis().try_into().unwrap_or(u64::MAX));
                        }
                    }
                    KeyAction::Skip => {
                        if let Some(ref pb) = pb {
                            pb.finish_with_message("Skipped!");
                        } else if raw && !quiet {
                            println!("\r{:10}", format_duration(Duration::ZERO));
                        }
                        return Ok(TimerOutcome::Completed);
                    }
                    KeyAction::Reset => {
                        start = Instant::now();
                        paused_accum = Duration::ZERO;
                        pause_start = None;
                        if let Some(ref pb) = pb {
                            pb.set_position(0);
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PomoState {
    Work(u32),
    ShortBreak(u32),
}

impl PomoState {
    pub fn next(self) -> Self {
        match self {
            PomoState::Work(cycle) => PomoState::ShortBreak(cycle),
            PomoState::ShortBreak(cycle) => PomoState::Work(cycle + 1),
        }
    }

    pub fn duration(&self, args: &Args) -> Duration {
        match self {
            PomoState::Work(_) => args.pomo_work,
            PomoState::ShortBreak(_) => args.pomo_break,
        }
    }

    pub fn completion_message(&self) -> String {
        match self {
            PomoState::Work(cycle) => format!("🍅 Pomodoro Cycle {}: Work finished!", cycle),
            PomoState::ShortBreak(cycle) => format!("☕ Pomodoro Cycle {}: Break finished!", cycle),
        }
    }

    pub fn announce(&self, args: &Args) {
        if !args.raw && !args.quiet {
            match self {
                PomoState::Work(cycle) => {
                    println!("🍅 Starting Pomodoro Cycle {} Work phase ({:?})...", cycle, args.pomo_work);
                }
                PomoState::ShortBreak(cycle) => {
                    println!("☕ Starting Pomodoro Cycle {} Break phase ({:?})...", cycle, args.pomo_break);
                }
            }
        }
    }
}

pub fn run_pomo_mode(args: &Args) -> Result<TimerOutcome, ZzzError> {
    let mut state = PomoState::Work(1);
    loop {
        state.announce(args);
        let outcome = run_sleep_timer(
            state.duration(args),
            args.quiet,
            args.raw,
            args.no_interaction,
            &args.theme,
            args.watch,
            &state.completion_message(),
            args.step,
        )?;

        if outcome != TimerOutcome::Completed {
            return Ok(outcome);
        }

        state = state.next();
    }
}

pub fn execute_command(cmd_str: &str) -> Result<(), ZzzError> {
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
            true,
            &Theme::Classic,
            None,
            "Done",
            Duration::from_secs(30),
        );
        assert_eq!(res.unwrap(), TimerOutcome::Completed);
    }

    #[test]
    fn test_pomo_state_transitions() {
        let state = PomoState::Work(1);
        assert_eq!(state.completion_message(), "🍅 Pomodoro Cycle 1: Work finished!");
        let next_state = state.next();
        assert_eq!(next_state, PomoState::ShortBreak(1));
        assert_eq!(next_state.completion_message(), "☕ Pomodoro Cycle 1: Break finished!");
        let next_work = next_state.next();
        assert_eq!(next_work, PomoState::Work(2));
    }

    #[test]
    fn test_run_sleep_timer_watched_dead_pid() {
        let res = run_sleep_timer(
            Duration::from_secs(10),
            true,
            false,
            true,
            &Theme::Classic,
            Some(999999),
            "Done",
            Duration::from_secs(30),
        );
        assert_eq!(res.unwrap(), TimerOutcome::WatchedProcessTerminated);
    }
}

