use crate::cli::Theme;
use indicatif::{ProgressBar, ProgressStyle};

pub fn create_progress_bar(total_millis: u64, theme: &Theme) -> Result<ProgressBar, Box<dyn std::error::Error>> {
    let pb = ProgressBar::new(total_millis);
    let style = match theme {
        Theme::Classic => ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg}",
        )?
        .progress_chars("██-"),
        Theme::Cat => ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.magenta/white}] {msg}",
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
            "{spinner} [{elapsed_precise}] [{bar:40.yellow/blue}] {msg}",
        )?
        .tick_strings(&["🌑 ", "🌒 ", "🌓 ", "🌔 ", "🌕 ", "🌖 ", "🌗 ", "🌘 "])
        .progress_chars("█>-"),
        Theme::Pixel => ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.green/black}] {msg}",
        )?
        .tick_strings(&["👾 ", "🕹️ ", "🎮 ", "👾 "])
        .progress_chars("▓▒░"),
        Theme::Matrix => ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.green/dim}] {msg}",
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
