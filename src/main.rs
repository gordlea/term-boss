use std::io::Write;

use clap::{Parser, Subcommand};
use config::{SetConfig, TermColorMapping, read_hierarchial_config};
use strum::VariantArray;

use crate::terminal_app::{PaneColor, TerminalApp};

mod colors;
mod config;
mod detect;
mod expand;
mod setup_shell;
mod scheme;
mod setup;
mod setup_colorschemes;
mod setup_vscode;
mod setup_wezterm;
mod show;
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
    /// Apply tab/pane settings from CLI flags, env vars and .term-boss.toml files
    Set(SetConfig),
    /// Explain what `set` would apply here and why, without changing the terminal
    Show(SetConfig),
    /// Print the shell hook that runs `term-boss set` on every directory change
    Env(setup_shell::EnvConfig),
    /// Install terminal-side integration (e.g. the wezterm Lua module)
    Setup(setup::SetupConfig),
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

            let mut config = read_hierarchial_config(cli_set_config)?;
            expand::expand_config(&mut config, &expand::Context { terminal: terminal_app });

            let badge_in_title = terminal_app.badge_in_title(config.badgetext.as_deref());
            let tabname = terminal_app.effective_title(config.tabname.as_deref(), config.badgetext.as_deref());

            if config.reset {
                // Only reset what is not about to be set, so a `cd` doesn't flicker
                // (and doesn't spawn `wezterm cli` twice).
                if config.tabcolor.is_none() {
                    terminal_app.reset_tab_color(&mut stdout);
                }
                if tabname.is_none() {
                    terminal_app.reset_tab_name(&mut stdout);
                }
                if config.badgetext.is_none() && terminal_app.supports_badge() {
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
                let mapped = config.color_mapping.as_ref();
                terminal_app.reset_palette_colors(
                    &mut stdout,
                    TermColorMapping::VARIANTS
                        .iter()
                        .copied()
                        .filter(|slot| !mapped.is_some_and(|m| m.contains_key(slot))),
                );
            }

            if let Some(tabcolor) = &config.tabcolor {
                terminal_app.set_tab_bg_color(&mut stdout, tabcolor);
            }
            if let (Some(badgetext), false) = (&config.badgetext, badge_in_title) {
                terminal_app.set_badge_text(&mut stdout, badgetext);
            }
            if let Some(tabname) = &tabname {
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
            for (slot, color) in config.color_mapping.iter().flatten() {
                terminal_app.set_palette_color(&mut stdout, *slot, color);
            }
        }
        Commands::Show(cli_set_config) => {
            show::run(cli_set_config, &mut stdout)?;
        }
        Commands::Env(env_config) => {
            env_config.shell.output(env_config.hook, &mut stdout);
        }
        Commands::Setup(setup_config) => {
            setup_config.install(&mut stdout)?;
        }
    }
    stdout.flush()?;
    Ok(())
}
