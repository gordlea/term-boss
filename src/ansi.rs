use std::io::Write;

use crate::colors::Color;
use base64::{engine::general_purpose::URL_SAFE, Engine};


pub fn set_tab_bg_color(stdout: &mut std::io::Stdout, tabcolor: &Color) {
    let color = tabcolor.get_rgb();

    // let red = tabcolor[0].to_string();
    let mut ansi = format!("\x1B]6;1;bg;red;brightness;{}\x07", color[0]).into_bytes();
    stdout.write(&ansi).unwrap();

    ansi = format!("\x1B]6;1;bg;green;brightness;{}\x07", color[1]).into_bytes();
    stdout.write(&ansi).unwrap();

    ansi = format!("\x1B]6;1;bg;blue;brightness;{}\x07", color[2]).into_bytes();
    stdout.write(&ansi).unwrap();
}

const RESET_BG_COLOR_ANSI: &'static str =  "\x1B]6;1;bg;*;default\x07";

pub fn reset_tab_color(stdout: &mut std::io::Stdout) {
    let ansi = RESET_BG_COLOR_ANSI.as_bytes();
    stdout.write(ansi).unwrap();
}

pub fn set_tab_name(stdout: &mut std::io::Stdout, tabname: &str) {
    let tabname = format!("\x1B]0;{}\x07", tabname).into_bytes();
    stdout.write(&tabname).unwrap();
}



const RESET_TAB_NAME: &'static str =  "\x1B]0;\x07";

pub fn reset_tab_name(stdout: &mut std::io::Stdout) {
    let ansi = RESET_TAB_NAME.as_bytes();
    stdout.write(ansi).unwrap();
}

pub fn set_badge_text(stdout: &mut std::io::Stdout, badgetext: &str) {
    let encoded_badgetext = URL_SAFE.encode(badgetext.as_bytes());
    let badgetext = format!("\x1B]1337;SetBadgeFormat={}\x07", encoded_badgetext).into_bytes();
    stdout.write(&badgetext).unwrap();
}

// #[derive(Debug, Clone, IntoStaticStr, VariantArray, Deserialize, Serialize)]
// #[repr(u8)]
// #[strum(ascii_case_insensitive, serialize_all = "lowercase")]
// #[serde(rename_all = "lowercase")]
// pub enum AnsiPaletteKey {
//     #[serde(rename(serialize = "fg", deserialize = "fg"))]
//     Foreground,
//     #[serde(rename(serialize = "bg", deserialize = "bg"))]
//     Background,
//     #[serde(rename(serialize = "bold", deserialize = "bold"))]
//     Bold,
//     #[serde(rename(serialize = "link", deserialize = "link"))]
//     Link,
//     #[serde(rename(serialize = "selfg", deserialize = "selfg"))]
//     SelectionForeground,
//     #[serde(rename(serialize = "selbg", deserialize = "selbg"))]
//     SelectionBackground,
//     #[serde(rename(serialize = "curfg", deserialize = "curfg"))]
//     CursorForeground,
//     #[serde(rename(serialize = "curbg", deserialize = "curbg"))]
//     CursorBackground,
//     #[serde(rename(serialize = "underline", deserialize = "underline"))]
//     Underline,
//     #[serde(rename(serialize = "tab", deserialize = "tab"))]
//     Tab,
//     #[serde(rename(serialize = "black", deserialize = "black"))]
//     Black,
//     #[serde(rename(serialize = "red", deserialize = "red"))]
//     Red,
//     #[serde(rename(serialize = "green", deserialize = "green"))]
//     Green,
//     #[serde(rename(serialize = "yellow", deserialize = "yellow"))]
//     Yellow,
//     #[serde(rename(serialize = "blue", deserialize = "blue"))]
//     Blue,
//     #[serde(rename(serialize = "magenta", deserialize = "magenta"))]
//     Magenta,
//     #[serde(rename(serialize = "cyan", deserialize = "cyan"))]
//     Cyan,
//     #[serde(rename(serialize = "white", deserialize = "white"))]
//     White,
//     #[serde(rename(serialize = "br_black", deserialize = "br_black"))]
//     BrightBlack,
//     #[serde(rename(serialize = "br_red", deserialize = "br_red"))]
//     BrightRed,
//     #[serde(rename(serialize = "br_green", deserialize = "br_green"))]
//     BrightGreen,
//     #[serde(rename(serialize = "br_yellow", deserialize = "br_yellow"))]
//     BrightYellow,
//     #[serde(rename(serialize = "br_blue", deserialize = "br_blue"))]
//     BrightBlue,
//     #[serde(rename(serialize = "br_magenta", deserialize = "br_magenta"))]
//     BrightMagenta,
//     #[serde(rename(serialize = "br_cyan", deserialize = "br_cyan"))]
//     BrightCyan,
//     #serde(rename(serialize = "br_white", deserialize = "br_white"))]
//     BrightWhite,
// }



// pub fn set_palette_color(stdout: &mut std::io::Stdout, p_key: &AnsiPaletteKey, color: &Color) {
//     let hex_color = format!("{color:x}");
//     let ansi = format!("\x1B]1337;SetColors={}={}\x07", serde_yml::to_string(p_key).unwrap(), hex_color).into_bytes();    
//     stdout.write(&ansi).unwrap();

// }