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
#[command(version)]
#[command(about = "A fancy sleep command written in Rust 💤", long_about = None)]
pub struct Args {
    /// Sleep duration (e.g., 5s, 1.5m, 2h)
    #[arg(value_parser = humantime::parse_duration, env = "ZZZ_DURATION")]
    pub duration: Option<Duration>,

    /// Sleep until a specific time (e.g., "17:00", "17:00:00", "tomorrow-8am", "tomorrow-17:00")
    #[arg(long, value_name = "TIME", env = "ZZZ_UNTIL")]
    pub until: Option<String>,

    /// Enable Pomodoro work/break timer cycles (e.g., 25m work / 5m break)
    #[arg(long, env = "ZZZ_POMO")]
    pub pomo: bool,

    /// Work duration for Pomodoro mode (default: 25m)
    #[arg(long, default_value = "25m", value_parser = humantime::parse_duration, env = "ZZZ_POMO_WORK")]
    pub pomo_work: Duration,

    /// Break duration for Pomodoro mode (default: 5m)
    #[arg(long, default_value = "5m", value_parser = humantime::parse_duration, env = "ZZZ_POMO_BREAK")]
    pub pomo_break: Duration,

    /// Progress bar theme
    #[arg(long, value_enum, default_value_t = Theme::Classic, env = "ZZZ_THEME")]
    pub theme: Theme,

    /// Execute command upon wake-up
    #[arg(long, alias = "exec", value_name = "COMMAND", env = "ZZZ_THEN")]
    pub then: Option<String>,

    /// Monitor PID and exit early if process terminates
    #[arg(long, value_name = "PID", env = "ZZZ_WATCH")]
    pub watch: Option<u32>,

    /// Enable quiet mode (hide progress bar)
    #[arg(short, long, env = "ZZZ_QUIET")]
    pub quiet: bool,

    /// Custom message displayed upon completion
    #[arg(short, long, default_value = "Woke up! 🚀", env = "ZZZ_MESSAGE")]
    pub message: String,

    /// Step duration for adjusting time during timer (+/- keys, default: 30s)
    #[arg(long, default_value = "30s", value_parser = humantime::parse_duration, env = "ZZZ_STEP")]
    pub step: Duration,

    /// Output only the time in a concise format
    #[arg(long, env = "ZZZ_RAW")]
    pub raw: bool,

    /// Disable keyboard interaction and terminal raw mode (for scripts/CI/cron)
    #[arg(long, env = "ZZZ_NO_INTERACTION")]
    pub no_interaction: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let args =
            Args::try_parse_from(&["zzz", "5s", "--theme", "cat", "--then", "echo hello"]).unwrap();
        assert_eq!(args.duration, Some(Duration::from_secs(5)));
        assert_eq!(args.theme, Theme::Cat);
        assert_eq!(args.then, Some("echo hello".to_string()));
        assert_eq!(args.step, Duration::from_secs(30));
        assert!(!args.raw);

        let exec_alias_args = Args::try_parse_from(&["zzz", "10s", "--exec", "ls"]).unwrap();
        assert_eq!(exec_alias_args.then, Some("ls".to_string()));

        let raw_step_args = Args::try_parse_from(&["zzz", "10s", "--raw", "--step", "1m"]).unwrap();
        assert!(raw_step_args.raw);
        assert_eq!(raw_step_args.step, Duration::from_secs(60));

        let no_interaction_args =
            Args::try_parse_from(&["zzz", "10s", "--no-interaction"]).unwrap();
        assert!(no_interaction_args.no_interaction);
    }

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_env_vars() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("ZZZ_THEME", "cat");
            std::env::set_var("ZZZ_MESSAGE", "Alarm ringing!");
            std::env::set_var("ZZZ_STEP", "1m");
            std::env::set_var("ZZZ_QUIET", "true");
            std::env::set_var("ZZZ_NO_INTERACTION", "true");
            std::env::set_var("ZZZ_THEN", "notify-send \"Done\"");
            std::env::set_var("ZZZ_POMO_WORK", "50m");
            std::env::set_var("ZZZ_POMO_BREAK", "10m");
        }

        let args = Args::try_parse_from(&["zzz", "5s"]).unwrap();

        unsafe {
            std::env::remove_var("ZZZ_THEME");
            std::env::remove_var("ZZZ_MESSAGE");
            std::env::remove_var("ZZZ_STEP");
            std::env::remove_var("ZZZ_QUIET");
            std::env::remove_var("ZZZ_NO_INTERACTION");
            std::env::remove_var("ZZZ_THEN");
            std::env::remove_var("ZZZ_POMO_WORK");
            std::env::remove_var("ZZZ_POMO_BREAK");
        }

        assert_eq!(args.theme, Theme::Cat);
        assert_eq!(args.message, "Alarm ringing!");
        assert_eq!(args.step, Duration::from_secs(60));
        assert!(args.quiet);
        assert!(args.no_interaction);
        assert_eq!(args.then, Some("notify-send \"Done\"".to_string()));
        assert_eq!(args.pomo_work, Duration::from_secs(50 * 60));
        assert_eq!(args.pomo_break, Duration::from_secs(10 * 60));
    }
}
