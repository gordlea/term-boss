use std::{ env, io::Write, path::PathBuf };
use ansi::{ reset_tab_color, reset_tab_name, set_badge_text, set_tab_bg_color, set_tab_name };
// use argh::FromArgs;
use colors::Color;
use clap::{ Parser, Subcommand };
use config::{ read_hierarchial_config, SetConfig };
use serde::Deserialize;

mod colors;
mod ansi;
mod config;
mod setup_shell;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// does testing things
    Set(SetConfig),
    Env(setup_shell::EnvConfig),
}

fn main() {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout();

    match &cli.command {
        Commands::Set(cli_set_config) => {
            let config = read_hierarchial_config(cli_set_config).unwrap();

            if config.reset {
                reset_tab_color(&mut stdout);
                reset_tab_name(&mut stdout);
                set_badge_text(&mut stdout, "");
            }

            if let Some(tabcolor) = config.tabcolor {
                set_tab_bg_color(&mut stdout, &tabcolor);
            }

            if let Some(badgetext) = config.badgetext {
                set_badge_text(&mut stdout, &badgetext);
            }

            if let Some(tabname) = config.tabname {
                set_tab_name(&mut stdout, &tabname);
            }

            // set_palette_color(&mut stdout, &AnsiPaletteKey::Foreground, &Color::White);

        }
        Commands::Env(env_config) => {
            env_config.shell.output(&mut stdout);
        },
    }
    let _ = stdout.flush();

}
