use std::fmt::Display;

use poise::serenity_prelude::Timestamp;

/// The format in which you want the timestamp to be generated.
#[allow(unused)]
pub enum Format {
    ShortTime,
    LongTime,

    ShortDate,
    LongDate,
    LongDateShortTime,
    LongDateDayAndShortTime,

    Relative,
}

#[allow(clippy::module_name_repetitions)]
pub trait TimestampExt {
    /// Converts a Serenity `Timestamp` into a Discord timestamp.
    ///
    /// Example:
    /// ```rs
    /// use serenity::model::timestamp::Timestamp;
    /// use crate::timestamp::Format;
    ///
    /// let serenity_timestamp = Timestamp::now();
    /// let discord_timestamp = serenity_timestamp.to_discord_timestamp(Format::LongDate);
    /// println!("{discord_timestamp}");
    /// ```
    fn to_discord_timestamp(&self, format: Format) -> String;
}

impl TimestampExt for Timestamp {
    fn to_discord_timestamp(&self, format: Format) -> String {
        let epoch = self.unix_timestamp();
        let format_string = format.to_string();
        format!("<t:{epoch}:{format_string}>")
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let format_string = match self {
            Format::ShortTime => "t",
            Format::LongTime => "T",
            Format::ShortDate => "d",
            Format::LongDate => "D",
            Format::LongDateShortTime => "f",
            Format::LongDateDayAndShortTime => "F",
            Format::Relative => "R",
        };
        write!(f, "{format_string}")
    }
}