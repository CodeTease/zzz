use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "zzz")]
#[command(about = "A fancy sleep command written in Rust 💤", long_about = None)]
struct Args {
    /// Sleep duration (e.g., 5s, 1.5m, 2h)
    #[arg(value_parser = humantime::parse_duration)]
    duration: Duration,

    /// Enable quiet mode (hide progress bar)
    #[arg(short, long)]
    quiet: bool,

    /// Custom message displayed upon completion
    #[arg(short, long, default_value = "Woke up! 🚀")]
    message: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let total_millis: u64 = args.duration.as_millis().try_into().unwrap_or(u64::MAX);

    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    let signal_handle = signals.handle();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        for _ in signals.forever() {
            if tx.send(()).is_err() {
                break;
            }
        }
    });

    if total_millis == 0 {
        signal_handle.close();
        return Ok(());
    }

    if args.quiet {
        let _ = rx.recv_timeout(args.duration);
        signal_handle.close();
        return Ok(());
    }

    let pb = ProgressBar::new(total_millis);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {elapsed}/{duration} ({eta}) {msg}",
        )
        .unwrap()
        .progress_chars("██-"),
    );

    let start = Instant::now();
    let duration = args.duration;
    let total_millis_128 = duration.as_millis();

    while start.elapsed() < duration {
        let elapsed = start.elapsed();
        let elapsed_millis: u64 = elapsed.as_millis().try_into().unwrap_or(u64::MAX);
        let elapsed_millis_128 = elapsed.as_millis();

        let status_emoji = if elapsed_millis_128 * 4 < total_millis_128 {
            "🥱 Yawning..."
        } else if elapsed_millis_128 * 4 < total_millis_128 * 3 {
            "😴 Sleeping soundly..."
        } else {
            "😳 Almost awake..."
        };

        pb.set_message(status_emoji);
        pb.set_position(std::cmp::min(elapsed_millis, total_millis));

        let remaining = duration.saturating_sub(elapsed);
        let sleep_time = std::cmp::min(Duration::from_millis(50), remaining);

        if rx.recv_timeout(sleep_time).is_ok() {
            pb.abandon_with_message("Cancelled! 🛑");
            signal_handle.close();
            return Ok(());
        }
    }

    pb.set_position(total_millis);
    pb.finish_with_message(args.message);
    signal_handle.close();

    Ok(())
}

