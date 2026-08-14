use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
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

    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = Arc::clone(&interrupted);
    let main_thread = thread::current();

    thread::spawn(move || {
        if let Ok(mut signals) = Signals::new([SIGINT, SIGTERM]) {
            if signals.forever().next().is_some() {
                interrupted_clone.store(true, Ordering::SeqCst);
                main_thread.unpark();
            }
        }
    });

    if args.quiet || total_millis == 0 {
        let start = Instant::now();
        while start.elapsed() < args.duration {
            if interrupted.load(Ordering::SeqCst) {
                return;
            }
            let remaining = args.duration.saturating_sub(start.elapsed());
            thread::park_timeout(remaining);
        }
        return;
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

    while start.elapsed() < duration {
        // If Ctrl+C was pressed, abort cleanly
        if interrupted.load(Ordering::SeqCst) {
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

        // Sleep up to 50ms or wake up immediately if signal unparks main thread
        let remaining = duration.saturating_sub(elapsed);
        let sleep_time = std::cmp::min(Duration::from_millis(50), remaining);
        thread::park_timeout(sleep_time);
    }

    if interrupted.load(Ordering::SeqCst) {
        pb.abandon_with_message("Cancelled! 🛑");
        return;
    }

    pb.set_position(total_millis);
    pb.finish_with_message(args.message);
}

