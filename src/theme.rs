use crate::cli::Theme;
use crate::error::ZzzError;
use indicatif::{ProgressBar, ProgressStyle};

fn is_no_color() -> bool {
    std::env::var_os("NO_COLOR").map_or(false, |val| !val.is_empty())
}

pub fn create_progress_bar(total_millis: u64, theme: &Theme) -> Result<ProgressBar, ZzzError> {
    let pb = ProgressBar::new(total_millis);
    let no_color = is_no_color();

    let style = match theme {
        Theme::Classic => {
            let template = if no_color {
                "{spinner} [{elapsed_precise}] [{bar:40}] {msg}"
            } else {
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg}"
            };
            ProgressStyle::with_template(template)?.progress_chars("██-")
        }
        Theme::Cat => {
            let template = if no_color {
                "{spinner} [{elapsed_precise}] [{bar:40}] {msg}"
            } else {
                "{spinner} [{elapsed_precise}] [{bar:40.magenta/white}] {msg}"
            };
            ProgressStyle::with_template(template)?
                .tick_strings(&[
                    "ฅ^•ﻌ•^ฅ 🧶 ",
                    " ฅ^•ﻌ•^ฅ🧶 ",
                    "  ฅ^•ﻌ•^ฅ🧶",
                    "   ฅ^•ﻌ•^ฅ ",
                    "  ฅ^•ﻌ•^ฅ🧶",
                    " ฅ^•ﻌ•^ฅ🧶 ",
                ])
                .progress_chars("=#-")
        }
        Theme::Moon => {
            let template = if no_color {
                "{spinner} [{elapsed_precise}] [{bar:40}] {msg}"
            } else {
                "{spinner} [{elapsed_precise}] [{bar:40.yellow/blue}] {msg}"
            };
            ProgressStyle::with_template(template)?
                .tick_strings(&["🌑 ", "🌒 ", "🌓 ", "🌔 ", "🌕 ", "🌖 ", "🌗 ", "🌘 "])
                .progress_chars("█>-")
        }
        Theme::Pixel => {
            let template = if no_color {
                "{spinner} [{elapsed_precise}] [{bar:40}] {msg}"
            } else {
                "{spinner:.green} [{elapsed_precise}] [{bar:40.green/black}] {msg}"
            };
            ProgressStyle::with_template(template)?
                .tick_strings(&["👾 ", "🕹️ ", "🎮 ", "👾 "])
                .progress_chars("▓▒░")
        }
        Theme::Matrix => {
            let template = if no_color {
                "{spinner} [{elapsed_precise}] [{bar:40}] {msg}"
            } else {
                "{spinner:.green} [{elapsed_precise}] [{bar:40.green/dim}] {msg}"
            };
            ProgressStyle::with_template(template)?
                .tick_strings(&["0101 ", "1010 ", "0011 ", "1100 ", "1001 "])
                .progress_chars("█▓▒")
        }
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
        let themes = [
            Theme::Classic,
            Theme::Cat,
            Theme::Moon,
            Theme::Pixel,
            Theme::Matrix,
        ];
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
    fn test_no_color_environment() {
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(create_progress_bar(1000, &Theme::Classic).is_ok());
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
        assert!(create_progress_bar(1000, &Theme::Classic).is_ok());
    }
}
