use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
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

    if total_millis == 0 {
        return Ok(());
    }

    // Initialize signals
    let mut signals = Signals::new([SIGINT, SIGTERM])?;

    let pb = if !args.quiet {
        let pb = ProgressBar::new(total_millis);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {elapsed}/{duration} ({eta}) {msg}",
            )?
            .progress_chars("██-"),
        );
        Some(pb)
    } else {
        None
    };

    let start = Instant::now();
    let duration = args.duration;

    loop {
        let elapsed = start.elapsed();
        if elapsed >= duration {
            break; // Sleep period complete
        }

        // 1. Check for incoming SIGINT / SIGTERM (non-blocking)
        if signals.pending().next().is_some() {
            if let Some(ref pb) = pb {
                pb.abandon_with_message("Cancelled! 🛑");
            }
            return Ok(());
        }

        // 2. Update the UI if a progress bar is present
        if let Some(ref pb) = pb {
            let elapsed_millis: u64 = elapsed.as_millis().try_into().unwrap_or(u64::MAX);
            let ratio = elapsed.as_secs_f64() / duration.as_secs_f64();
            
            let status_emoji = if ratio < 0.25 {
                "🥱 Yawning..."
            } else if ratio < 0.75 {
                "😴 Sleeping soundly..."
            } else {
                "😳 Almost awake..."
            };

            pb.set_message(status_emoji);
            pb.set_position(elapsed_millis);
        }

        // 3. Brief sleep between UI updates
        let remaining = duration.saturating_sub(elapsed);
        std::thread::sleep(std::cmp::min(Duration::from_millis(50), remaining));
    }

    if let Some(pb) = pb {
        pb.set_position(total_millis);
        pb.finish_with_message(args.message);
    }

    Ok(())
}

