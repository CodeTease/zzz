use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

fn main() {
    let args = Args::parse();
    let total_millis = args.duration.as_millis() as u64;

    if args.quiet || total_millis == 0 {
        thread::sleep(args.duration);
        return;
    }

    // Flag to track interruption status on Ctrl+C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Catch Ctrl+C signals gracefully
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler!");

    let pb = ProgressBar::new(total_millis);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos_ms}/{len_ms} ({eta}) {msg}",
        )
        .unwrap()
        .progress_chars("██-"),
    );

    let start = Instant::now();
    let duration = args.duration;

    while start.elapsed() < duration {
        // If Ctrl+C was pressed, abort cleanly
        if !running.load(Ordering::SeqCst) {
            pb.abandon_with_message("Cancelled! 🛑");
            return;
        }

        let elapsed = start.elapsed();
        let elapsed_millis = elapsed.as_millis() as u64;
        let progress_ratio = elapsed.as_secs_f64() / duration.as_secs_f64();

        // Dynamic status emoji based on progress
        let status_emoji = match progress_ratio {
            p if p < 0.25 => "🥱 Yawning...",
            p if p < 0.75 => "😴 Sleeping soundly...",
            _ => "😳 Almost awake...",
        };

        pb.set_message(status_emoji);
        pb.set_position(std::cmp::min(elapsed_millis, total_millis));

        // Short sleep duration without overshooting the remaining time
        let remaining = duration.saturating_sub(elapsed);
        thread::sleep(std::cmp::min(Duration::from_millis(50), remaining));
    }

    pb.set_position(total_millis);
    pb.finish_with_message(args.message);
}
