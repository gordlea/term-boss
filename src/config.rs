use std::{env, fs};
use clap::Args;
use serde::Deserialize;
use anyhow::{Context, Result};
use crate::colors::Color;


const CONFIG_FILE_NAME: &'static str = ".iterm-tab.yml";




#[derive(Debug, Deserialize, Default, Args, Clone)]
/// Reach new heights.
#[command(version, about, long_about = None)]
pub struct SetConfig {
    // #[argh(option, description = "the new name for the current tab")]
    #[arg(long, env = "ITERM_TAB_NAME")]
    pub tabname: Option<String>,

    /// What badgetext you would like. Works with emojis. Can use variables from iterm
    /// by wrapping in single quotes. Example `--badgetext '\(hostname)'`
    /// See: https://iterm2.com/documentation-variables.html
    #[arg(long, env = "ITERM_TAB_BADGETEXT")]
    pub badgetext: Option<String>,

    /// The background color to set the tab to.
    #[arg(long, value_enum, env = "ITERM_TAB_BG_COLOR")]
    pub tabcolor: Option<Color>,

    /// If this is set, any options not specified will be reset to default settings
    #[arg(long, env = "ITERM_TAB_RESET")]
    #[serde(default)]
    pub reset: bool,
}

impl SetConfig {
    pub fn merge(&mut self, other: SetConfig, replace_existing_values: bool) {
        if other.tabname.is_some() && (replace_existing_values || self.tabname.is_none()) {
            self.tabname = other.tabname;
        }
        if other.badgetext.is_some() && (replace_existing_values || self.badgetext.is_none()) {
            self.badgetext = other.badgetext;
        }
        if other.tabcolor.is_some() && (replace_existing_values || self.tabcolor.is_none()) {
            self.tabcolor = other.tabcolor;
        }
    }
}




pub fn read_hierarchial_config(cli_set_config: &SetConfig) -> Result<SetConfig> {
    // Start from the current directory
    let mut current_dir = env::current_dir().with_context(|| format!("Unable to determine current dir!"))?;
    // eprintln!("Current dir: {:?}", current_dir);
    // let mut config_files: Vec<Config> = Vec::with_capacity(3);


    let mut merged_config = cli_set_config.clone();


    // Loop until we can no longer go up (or find the file)
    loop {
        let candidate = current_dir.join(CONFIG_FILE_NAME);
        if candidate.exists() {
            let candidate_contents = fs::read_to_string(candidate.clone());
            if let Ok(contents) = candidate_contents {
                let read_config = serde_yml::from_str(&contents).expect(format!("Unable to read file {} as a valid config file", CONFIG_FILE_NAME).as_str());
                merged_config.merge(read_config, false);
            } else {
                eprintln!("Unable to read file {:?} as a valid config file, skipping", candidate);
            }
        }

        // If `pop` returns false, it means we've reached the root
        if !current_dir.pop() {
            break;
        }
    }

    // let cli_and_env_config = ;
    // merged_config.merge(cli_and_env_config, false);

    // the shallower in the list, the lower priority, start there
    // eprintln!("Merged config: {:?}", merged_config);

    Ok(merged_config)
}


