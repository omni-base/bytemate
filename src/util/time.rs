use poise::serenity_prelude::Timestamp;
use regex::Regex;


pub(crate) fn parse_to_time(time_str: String) -> Option<u64> {
    let re = Regex::new(r"^(\d+)([dhms])$").unwrap();
    let captures = re.captures(&time_str)?;

    let time_value = captures[1].parse::<u64>().ok()?;
    let time_unit = &captures[2];

    match time_unit {
        "d" => Some(time_value * 86400),
        "h" => Some(time_value * 3600),
        "m" => Some(time_value * 60),
        "s" => Some(time_value),
        _ => None,
    }
}

pub(crate) fn date_after(time: u64) -> Timestamp {
    let now = chrono::Utc::now();
    let future = now + chrono::Duration::seconds(time as i64);
    future.into()
}