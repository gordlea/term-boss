use std::{io::Write, process::{Command, Stdio}};

use base64::{engine::general_purpose::URL_SAFE, Engine};

use crate::colors::Color;

const ITERM2_RESET_BG_COLOR_ANSI: &'static str =  "\x1B]6;1;bg;*;default\x07";
const ITERM2_RESET_TAB_NAME: &'static str =  "\x1B]0;\x07";
const WEZTERM_RESET_BG_COLOR_ANSI: &'static str =  "\x1B]1337;SetUserVar=TAB_COLOR=#ff5500\x07";


fn wezterm_cli_runner(args: &[&str]) {
    // let mut command_args: Vec<&str> = Vec::with_capacity(args.len() + 1);
    // command_args.extend_from_slice(args);

    let result = Command::new("wezterm")
        .arg("cli")
        .args(args)
        .output();
        // .stdin(Stdio::null())
        // .stdout(Stdio::null())
        // .stderr(Stdio::null())
        // .spawn();
    if let Err(e) = result {
        eprintln!("Failed to run wezterm command: {:?}", e);
    } else if let Ok(output) = result {
            eprintln!("Wezterm command worked with output: {:?}", output);
    }
}


#[derive(Debug, PartialEq, Eq, Default)]
pub enum TerminalApp {
    #[default]
    Unknown,
    Iterm2,
    Alacritty,
    Wezterm,
    Konsole,
}

impl TerminalApp {
    pub fn reset_tab_color(&self, stdout: &mut std::io::Stdout) {
        match self {
            
            TerminalApp::Iterm2 => {
                let ansi = ITERM2_RESET_BG_COLOR_ANSI.as_bytes();
                stdout.write(ansi).unwrap();
            },
            TerminalApp::Alacritty => todo!(),
            TerminalApp::Wezterm => {
                let ansi = WEZTERM_RESET_BG_COLOR_ANSI.as_bytes();
                stdout.write(ansi).unwrap();
            },
            TerminalApp::Konsole => todo!(),
            TerminalApp::Unknown => { /* no op */ },
        }
    }

    pub fn reset_tab_name(&self, stdout: &mut std::io::Stdout) {
        match self {
            TerminalApp::Iterm2 => {
                let ansi = ITERM2_RESET_TAB_NAME.as_bytes();
                stdout.write(ansi).unwrap();
            },
            TerminalApp::Alacritty => todo!(),
            TerminalApp::Wezterm => {
                wezterm_cli_runner(&["set-tab-title", ""]);
            },
            TerminalApp::Konsole => todo!(),
            TerminalApp::Unknown => { /* no op */ },
        }
    }


    pub fn set_badge_text(&self, stdout: &mut std::io::Stdout, badgetext: &str) {
        match self {
            TerminalApp::Iterm2 => {
                let encoded_badgetext = URL_SAFE.encode(badgetext.as_bytes());
                let badgetext = format!("\x1B]1337;SetBadgeFormat={}\x07", encoded_badgetext).into_bytes();
                stdout.write(&badgetext).unwrap();
            },
            TerminalApp::Alacritty => todo!(),
            TerminalApp::Wezterm => {},
            TerminalApp::Konsole => todo!(),
            TerminalApp::Unknown => { /* no op */ },
        }
    }

    pub fn set_tab_bg_color(&self, stdout: &mut std::io::Stdout, tabcolor: &Color) {
        match self {
            TerminalApp::Iterm2 => {
                let color = tabcolor.get_rgb();

                // let red = tabcolor[0].to_string();
                let mut ansi = format!("\x1B]6;1;bg;red;brightness;{}\x07", color[0]).into_bytes();
                stdout.write(&ansi).unwrap();

                ansi = format!("\x1B]6;1;bg;green;brightness;{}\x07", color[1]).into_bytes();
                stdout.write(&ansi).unwrap();

                ansi = format!("\x1B]6;1;bg;blue;brightness;{}\x07", color[2]).into_bytes();
                stdout.write(&ansi).unwrap();
            },
            TerminalApp::Alacritty => todo!(),
            TerminalApp::Wezterm => {
                let color = tabcolor.get_rgb();
                let value = format!("\x1B]1337;SetUserVar=TAB_COLOR=rgb({},{},{})\x07", color[0], color[1], color[2]);
                let ansi = value.as_bytes();
                stdout.write(ansi).unwrap();                
            },
            TerminalApp::Konsole => todo!(),
            TerminalApp::Unknown => { /* no op */ },
        }
    }

    pub fn set_tab_name(&self, stdout: &mut std::io::Stdout, tabname: &str) {
        eprintln!("Setting tab name to: {}", tabname);

        match self {
            TerminalApp::Iterm2 => {
                let tabname = format!("\x1B]0;{}\x07", tabname).into_bytes();
                stdout.write(&tabname).unwrap();
            },
            TerminalApp::Alacritty => todo!(),
            TerminalApp::Wezterm => {
                wezterm_cli_runner(&["set-tab-title", tabname]);
            },
            TerminalApp::Konsole => {

            },
            TerminalApp::Unknown => { /* no op */ },
        }
    }        
}