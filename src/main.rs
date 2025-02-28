use std::io::Write;
// use argh::FromArgs;
use colors::Color;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use clap::Parser;

mod colors;



#[derive(Debug, Parser)]
/// Reach new heights.
#[command(version, about, long_about = None)]
struct ItermViewSettings {
    // #[argh(option, description = "the new name for the current tab")]
    #[arg(long, env = "ITERM_BOSS_TABNAME")]
    tabname: Option<String>,

    // #[argh(option, description = "the badge text to display")]
    #[arg(long, env = "ITERM_BOSS_BADGETEXT")]
    badgetext: Option<String>,

    // #[argh(option, description = "the new color for the current tab", from_str_fn(parse_color_option))]
    #[arg(long, value_enum, env = "ITERM_BOSS_TABCOLOR")]
    tabcolor: Option<Color>,
}

impl ItermViewSettings {
    fn apply(&self) {
        let mut stdout = std::io::stdout();

        if let Some(tabcolor) = &self.tabcolor {
            let color = tabcolor.get_rgb();

            // let red = tabcolor[0].to_string();
            let red = format!("\x1B]6;1;bg;red;brightness;{}\x07", color[0]).into_bytes();
            let green = format!("\x1B]6;1;bg;green;brightness;{}\x07", color[1]).into_bytes();
            let blue = format!("\x1B]6;1;bg;blue;brightness;{}\x07", color[2]).into_bytes();

            stdout.write(&red).unwrap();
            stdout.write(&green).unwrap();
            stdout.write(&blue).unwrap();
            
        }

        if let Some(badgetext) = &self.badgetext {
            let encoded_badgetext = URL_SAFE.encode(badgetext.as_bytes());
            let badgetext = format!("\x1B]1337;SetBadgeFormat={}\x07", encoded_badgetext).into_bytes();
            stdout.write(&badgetext).unwrap();
        }

        if let Some(tabname) = &self.tabname {
            let tabname = format!("\x1B]0;{}\x07", tabname).into_bytes();
            stdout.write(&tabname).unwrap();
        }

        let _ = stdout.flush();

    }

}

// fn parse_color_option(value: &str) -> std::result::Result<[u8; 3], String> {
//     let color_result= Color::from_str(value);
//     if let Ok(color) = color_result {
//         return Ok(color.get_rgb());
//     }
//     let e = color_result.unwrap_err();
//     return Err(e.to_string());
// }

fn main() {
    let settings: ItermViewSettings = ItermViewSettings::parse();
    // eprintln!("TabOptions: {:?}", settings);
    // let mut stdout = std::io::stdout();
    settings.apply();
    
    // print!("\x1b[6;1;bg;red;brightness;225");
}