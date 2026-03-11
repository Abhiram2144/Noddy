#[derive(Debug, Clone)]
pub struct ScheduleEntry {
    pub day: String,
    pub subject: String,
    pub time: String,
}

pub fn parse_bulk_schedule_input(text: &str) -> Vec<ScheduleEntry> {
    let mut entries = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (day_raw, body) = match trimmed.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };

        let day = normalize_day(day_raw);
        if day.is_empty() {
            continue;
        }

        for item in body.split(',') {
            let chunk = item.trim();
            if chunk.is_empty() {
                continue;
            }

            if let Some((subject, time)) = split_subject_time(chunk) {
                entries.push(ScheduleEntry {
                    day: day.clone(),
                    subject,
                    time,
                });
            }
        }
    }

    entries
}

fn normalize_day(value: &str) -> String {
    let lower = value.trim().to_lowercase();
    match lower.as_str() {
        "mon" | "monday" => "monday".to_string(),
        "tue" | "tues" | "tuesday" => "tuesday".to_string(),
        "wed" | "wednesday" => "wednesday".to_string(),
        "thu" | "thurs" | "thursday" => "thursday".to_string(),
        "fri" | "friday" => "friday".to_string(),
        "sat" | "saturday" => "saturday".to_string(),
        "sun" | "sunday" => "sunday".to_string(),
        _ => String::new(),
    }
}

fn split_subject_time(chunk: &str) -> Option<(String, String)> {
    let tokens = chunk.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }

    for i in 0..tokens.len() {
        let normalized = tokens[i]
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != ':')
            .to_lowercase();

        let mut parsed_time: Option<String> = None;
        if normalized.ends_with("am") || normalized.ends_with("pm") {
            parsed_time = normalize_time_token(&normalized);
        } else if i + 1 < tokens.len() {
            let next = tokens[i + 1]
                .trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_lowercase();
            if (next == "am" || next == "pm")
                && normalized.chars().all(|c| c.is_ascii_digit() || c == ':')
            {
                parsed_time = normalize_time_token(&format!("{}{}", normalized, next));
            }
        }

        if let Some(time) = parsed_time {
            let subject = tokens[..i].join(" ").trim().to_string();
            if !subject.is_empty() {
                return Some((subject, time));
            }
        }
    }

    None
}

fn normalize_time_token(raw: &str) -> Option<String> {
    let lower = raw.to_lowercase().replace(' ', "");
    let suffix = if lower.ends_with("am") {
        "AM"
    } else if lower.ends_with("pm") {
        "PM"
    } else {
        return None;
    };

    let number = lower.trim_end_matches("am").trim_end_matches("pm");
    if number.is_empty() {
        return None;
    }

    if let Some((h, m)) = number.split_once(':') {
        let hour = h.parse::<u32>().ok()?;
        let min = m.parse::<u32>().ok()?;
        if hour == 0 || hour > 12 || min > 59 {
            return None;
        }
        return Some(format!("{}:{:02} {}", hour, min, suffix));
    }

    let hour = number.parse::<u32>().ok()?;
    if hour == 0 || hour > 12 {
        return None;
    }
    Some(format!("{}:00 {}", hour, suffix))
}
