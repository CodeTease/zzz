use chrono::{Duration as ChronoDuration, Local, NaiveTime};
use clap::{Parser, ValueEnum};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use indicatif::{ProgressBar, ProgressStyle};
use signal_hook::consts::signal::{SIGCONT, SIGINT, SIGTERM, SIGTSTP};
use signal_hook::iterator::Signals;
use std::process::Command;
use std::time::{Duration, Instant};

struct RawModeGuard {
    active: bool,
}

impl RawModeGuard {
    fn new() -> Self {
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

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum Theme {
    Classic,
    Cat,
    Moon,
    Pixel,
    Matrix,
}

#[derive(Parser, Debug)]
#[command(name = "zzz")]
#[command(about = "A fancy sleep command written in Rust 💤", long_about = None)]
pub struct Args {
    /// Sleep duration (e.g., 5s, 1.5m, 2h)
    #[arg(value_parser = humantime::parse_duration)]
    pub duration: Option<Duration>,

    /// Sleep until a specific time (e.g., "17:00", "17:00:00", "tomorrow-8am", "tomorrow-17:00")
    #[arg(long, value_name = "TIME")]
    pub until: Option<String>,

    /// Enable Pomodoro work/break timer cycles (e.g., 25m work / 5m break)
    #[arg(long)]
    pub pomo: bool,

    /// Work duration for Pomodoro mode (default: 25m)
    #[arg(long, default_value = "25m", value_parser = humantime::parse_duration)]
    pub pomo_work: Duration,

    /// Break duration for Pomodoro mode (default: 5m)
    #[arg(long, default_value = "5m", value_parser = humantime::parse_duration)]
    pub pomo_break: Duration,

    /// Progress bar theme
    #[arg(long, value_enum, default_value_t = Theme::Classic)]
    pub theme: Theme,

    /// Execute command upon wake-up
    #[arg(long, alias = "exec", value_name = "COMMAND")]
    pub then: Option<String>,

    /// Monitor PID and exit early if process terminates
    #[arg(long, value_name = "PID")]
    pub watch: Option<u32>,

    /// Enable quiet mode (hide progress bar)
    #[arg(short, long)]
    pub quiet: bool,

    /// Custom message displayed upon completion
    #[arg(short, long, default_value = "Woke up! 🚀")]
    pub message: String,
}

pub fn create_progress_bar(total_millis: u64, theme: &Theme) -> Result<ProgressBar, Box<dyn std::error::Error>> {
    let pb = ProgressBar::new(total_millis);
    let style = match theme {
        Theme::Classic => ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {elapsed}/{duration} ({eta}) {msg}",
        )?
        .progress_chars("██-"),
        Theme::Cat => ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.magenta/white}] {elapsed}/{duration} ({eta}) {msg}",
        )?
        .tick_strings(&[
            "ฅ^•ﻌ•^ฅ 🧶 ",
            " ฅ^•ﻌ•^ฅ🧶 ",
            "  ฅ^•ﻌ•^ฅ🧶",
            "   ฅ^•ﻌ•^ฅ ",
            "  ฅ^•ﻌ•^ฅ🧶",
            " ฅ^•ﻌ•^ฅ🧶 ",
        ])
        .progress_chars("=#-"),
        Theme::Moon => ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.yellow/blue}] {elapsed}/{duration} ({eta}) {msg}",
        )?
        .tick_strings(&["🌑 ", "🌒 ", "🌓 ", "🌔 ", "🌕 ", "🌖 ", "🌗 ", "🌘 "])
        .progress_chars("█>-"),
        Theme::Pixel => ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.green/black}] {elapsed}/{duration} ({eta}) {msg}",
        )?
        .tick_strings(&["👾 ", "🕹️ ", "🎮 ", "👾 "])
        .progress_chars("▓▒░"),
        Theme::Matrix => ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.green/dim}] {elapsed}/{duration} ({eta}) {msg}",
        )?
        .tick_strings(&["0101 ", "1010 ", "0011 ", "1100 ", "1001 "])
        .progress_chars("█▓▒"),
    };
    pb.set_style(style);
    Ok(pb)
}

pub fn get_status_frame(ratio: f64) -> &'static str {
    if ratio < 0.25 {
        "😴 ( z z Z ) Sleeping soundly..."
    } else if ratio < 0.50 {
        "🥱 ( ~ ~ ~ ) Yawning..."
    } else if ratio < 0.75 {
        "😳 ( o . o ) Almost awake..."
    } else {
        "⚡ └(^o^)┘ WAKE UP!"
    }
}

pub fn parse_until_target(input: &str) -> Result<Duration, String> {
    let now = Local::now();
    let s = input.trim().to_lowercase();

    let (is_tomorrow, time_str) = if s.starts_with("tomorrow-") {
        (true, s.trim_start_matches("tomorrow-"))
    } else if s.starts_with("tomorrow ") {
        (true, s.trim_start_matches("tomorrow "))
    } else {
        (false, s.as_str())
    };

    let target_time = parse_time_str(time_str)?;

    let mut target_datetime = now.date_naive().and_time(target_time);

    if is_tomorrow {
        target_datetime += ChronoDuration::days(1);
    } else {
        if target_datetime <= now.naive_local() {
            target_datetime += ChronoDuration::days(1);
        }
    }

    let diff = target_datetime.signed_duration_since(now.naive_local());
    if diff.num_milliseconds() <= 0 {
        return Ok(Duration::ZERO);
    }

    Ok(Duration::from_millis(diff.num_milliseconds() as u64))
}

fn parse_time_str(s: &str) -> Result<NaiveTime, String> {
    let s_clean = s.trim();

    if let Ok(t) = NaiveTime::parse_from_str(s_clean, "%H:%M:%S") {
        return Ok(t);
    }
    if let Ok(t) = NaiveTime::parse_from_str(s_clean, "%H:%M") {
        return Ok(t);
    }

    let upper = s_clean.to_uppercase();
    if let Some(rest) = upper.strip_suffix("AM").or_else(|| upper.strip_suffix("PM")) {
        let is_pm = upper.ends_with("PM");
        let parts: Vec<&str> = rest.split(':').collect();
        let hour: u32 = parts.get(0).and_then(|h| h.parse().ok()).ok_or_else(|| {
            format!("Invalid hour in time string: '{}'", s_clean)
        })?;
        let minute: u32 = if parts.len() > 1 {
            parts[1].parse().map_err(|_| format!("Invalid minute in time string: '{}'", s_clean))?
        } else {
            0
        };
        let second: u32 = if parts.len() > 2 {
            parts[2].parse().map_err(|_| format!("Invalid second in time string: '{}'", s_clean))?
        } else {
            0
        };

        let hour_24 = match (is_pm, hour) {
            (true, 12) => 12,
            (true, h) if h < 12 => h + 12,
            (false, 12) => 0,
            (false, h) if h < 12 => h,
            _ => return Err(format!("Invalid hour: {}", hour)),
        };

        if let Some(t) = NaiveTime::from_hms_opt(hour_24, minute, second) {
            return Ok(t);
        }
    }

    Err(format!(
        "Invalid time format: '{}'. Supported formats: '17:00', '17:00:00', '5pm', '5:30pm', 'tomorrow-8am', 'tomorrow-17:00'",
        s
    ))
}

pub fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if res == 0 {
            true
        } else {
            std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
        }
    }
    #[cfg(not(unix))]
    {
        true
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

    let _raw_mode_guard = RawModeGuard::new();

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

        // Handle crossterm keypresses
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key_event) = event::read()? {
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

            if pause_start.is_some() {
                pb.set_message("⏸️ Paused (Press Space/P to resume)");
            } else {
                pb.set_message(get_status_frame(ratio));
            }
            pb.set_position(elapsed_millis);
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

fn execute_command(cmd_str: &str) -> Result<(), Box<dyn std::error::Error>> {
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
        parse_until_target(until_str).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
    } else {
        eprintln!("Error: Please specify a duration (e.g. `zzz 10s`), `--until <TIME>`, or `--pomo`.");
        std::process::exit(1);
    };

    let completed = run_sleep_timer(duration, args.quiet, &args.theme, args.watch, &args.message)?;

    if completed {
        if let Some(ref cmd) = args.then {
            execute_command(cmd)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_str() {
        assert_eq!(
            parse_time_str("17:00").unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap()
        );
        assert_eq!(
            parse_time_str("5pm").unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap()
        );
        assert_eq!(
            parse_time_str("8am").unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap()
        );
        assert_eq!(
            parse_time_str("08:30am").unwrap(),
            NaiveTime::from_hms_opt(8, 30, 0).unwrap()
        );
    }

    #[test]
    fn test_parse_until_target() {
        assert!(parse_until_target("23:59").is_ok());
        assert!(parse_until_target("tomorrow-8am").is_ok());
    }

    #[test]
    fn test_themes_creation() {
        let themes = [Theme::Classic, Theme::Cat, Theme::Moon, Theme::Pixel, Theme::Matrix];
        for t in &themes {
            assert!(create_progress_bar(1000, t).is_ok());
        }
    }

    #[test]
    fn test_status_frame() {
        assert_eq!(get_status_frame(0.1), "😴 ( z z Z ) Sleeping soundly...");
        assert_eq!(get_status_frame(0.3), "🥱 ( ~ ~ ~ ) Yawning...");
        assert_eq!(get_status_frame(0.6), "😳 ( o . o ) Almost awake...");
        assert_eq!(get_status_frame(0.9), "⚡ └(^o^)┘ WAKE UP!");
    }

    #[test]
    fn test_is_process_running() {
        let my_pid = std::process::id();
        assert!(is_process_running(my_pid));
        assert!(!is_process_running(999999));
    }

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

    #[test]
    fn test_cli_parsing() {
        let args = Args::try_parse_from(&["zzz", "5s", "--theme", "cat", "--then", "echo hello"]).unwrap();
        assert_eq!(args.duration, Some(Duration::from_secs(5)));
        assert_eq!(args.theme, Theme::Cat);
        assert_eq!(args.then, Some("echo hello".to_string()));

        let exec_alias_args = Args::try_parse_from(&["zzz", "10s", "--exec", "ls"]).unwrap();
        assert_eq!(exec_alias_args.then, Some("ls".to_string()));
    }
}

