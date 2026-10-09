use std::io::Write;

use clap::{Parser, Subcommand};
use config::{SetConfig, read_hierarchial_config};

use crate::terminal_app::{PaneColor, TerminalApp};

mod colors;
mod config;
mod detect;
mod setup_shell;
mod setup_wezterm;
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
    /// Apply tab/pane settings from CLI flags, env vars and .term-boss.yml files
    Set(SetConfig),
    /// Print the shell hook that runs `term-boss set` on every directory change
    Env(setup_shell::EnvConfig),
    /// Install terminal-side integration (e.g. the wezterm Lua module)
    Setup(setup_wezterm::SetupConfig),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout();

    match &cli.command {
        Commands::Set(cli_set_config) => {
            let terminal_app = detect::detect_terminal();
            if terminal_app == TerminalApp::Unknown {
                return Ok(());
            }

            let config = read_hierarchial_config(cli_set_config)?;

            if config.reset {
                // Only reset what is not about to be set, so a `cd` doesn't flicker
                // (and doesn't spawn `wezterm cli` twice).
                if config.tabcolor.is_none() {
                    terminal_app.reset_tab_color(&mut stdout);
                }
                if config.tabname.is_none() {
                    terminal_app.reset_tab_name(&mut stdout);
                }
                if config.badgetext.is_none() {
                    terminal_app.set_badge_text(&mut stdout, "");
                }
                if config.fgcolor.is_none() {
                    terminal_app.reset_pane_color(&mut stdout, PaneColor::Foreground);
                }
                if config.bgcolor.is_none() {
                    terminal_app.reset_pane_color(&mut stdout, PaneColor::Background);
                }
                if config.cursorcolor.is_none() {
                    terminal_app.reset_pane_color(&mut stdout, PaneColor::Cursor);
                }
            }

            if let Some(tabcolor) = &config.tabcolor {
                terminal_app.set_tab_bg_color(&mut stdout, tabcolor);
            }
            if let Some(badgetext) = &config.badgetext {
                terminal_app.set_badge_text(&mut stdout, badgetext);
            }
            if let Some(tabname) = &config.tabname {
                terminal_app.set_tab_name(&mut stdout, tabname);
            }
            if let Some(color) = &config.fgcolor {
                terminal_app.set_pane_color(&mut stdout, PaneColor::Foreground, color);
            }
            if let Some(color) = &config.bgcolor {
                terminal_app.set_pane_color(&mut stdout, PaneColor::Background, color);
            }
            if let Some(color) = &config.cursorcolor {
                terminal_app.set_pane_color(&mut stdout, PaneColor::Cursor, color);
            }
        }
        Commands::Env(env_config) => {
            env_config.shell.output(&mut stdout);
        }
        Commands::Setup(setup_config) => {
            setup_config.target.install(&mut stdout)?;
        }
    }
    stdout.flush()?;
    Ok(())
}
