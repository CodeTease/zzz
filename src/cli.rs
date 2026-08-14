use clap::{Parser, ValueEnum};
use std::time::Duration;

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

#[cfg(test)]
mod tests {
    use super::*;

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
