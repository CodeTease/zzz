use chrono::{Duration as ChronoDuration, Local, NaiveTime};
use std::time::Duration;

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
        let parts: Vec<&str> = rest.trim().split(':').collect();
        let hour: u32 = parts.get(0).and_then(|h| h.trim().parse().ok()).ok_or_else(|| {
            format!("Invalid hour in time string: '{}'", s_clean)
        })?;
        let minute: u32 = if parts.len() > 1 {
            parts[1].trim().parse().map_err(|_| format!("Invalid minute in time string: '{}'", s_clean))?
        } else {
            0
        };
        let second: u32 = if parts.len() > 2 {
            parts[2].trim().parse().map_err(|_| format!("Invalid second in time string: '{}'", s_clean))?
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
            parse_time_str("5 pm").unwrap(),
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
}
