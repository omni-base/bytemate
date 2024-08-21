use poise::serenity_prelude::Colour;


pub(crate) fn hex(hex: &str) -> Colour {
    let hex = hex.trim_start_matches("#");
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    Colour::from_rgb(r, g, b)
}

pub(crate) enum BotColors {
    Default,
}

impl BotColors {
    pub(crate) fn color(&self) -> Colour {
        match self {
            BotColors::Default => hex("#5848CE"),
        }
    }
}