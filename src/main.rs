use std:: io::Write;
// use argh::FromArgs;
use clap::{ Parser, Subcommand };
use config::{ read_hierarchial_config, SetConfig };

use crate::terminal_app::TerminalApp;

mod colors;
mod config;
mod setup_shell;
mod detect;
mod terminal_app;

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

    let terminal_app = detect::detect_terminal();
    if let TerminalApp::Unknown = terminal_app {
        eprintln!("Could not detect terminal application. No Op.");
        return;
    }

    eprintln!("Detected terminal: {:?}", terminal_app);

    match &cli.command {
        Commands::Set(cli_set_config) => {
            let config = read_hierarchial_config(cli_set_config).unwrap();
            eprintln!("config: {:?}", config);

            if config.reset {
                terminal_app.reset_tab_color(&mut stdout);
                terminal_app.reset_tab_name(&mut stdout);
                terminal_app.set_badge_text(&mut stdout, "");
            }

            if let Some(tabcolor) = config.tabcolor {
                terminal_app.set_tab_bg_color(&mut stdout, &tabcolor);
            }

            if let Some(badgetext) = config.badgetext {
                terminal_app.set_badge_text(&mut stdout, &badgetext);
            }

            if let Some(tabname) = config.tabname {
                terminal_app.set_tab_name(&mut stdout, &tabname);
            }

            // set_palette_color(&mut stdout, &AnsiPaletteKey::Foreground, &Color::White);

        }
        Commands::Env(env_config) => {
            env_config.shell.output(&mut stdout);
        },
    }
    let _ = stdout.flush();

}
