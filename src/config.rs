use std::path::PathBuf;

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};

use crate::PROJECT_NAME;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    // The browser executable name
    pub browser_name: String,
    pub kill_tab_strategies: Vec<KillTabStrategy>,
    // The interval of applying strategy, in secs
    pub check_interval_secs: f32,
    // Don't discard if the tab produce sound recently
    pub whitelist_audible: bool,
    // A list of regex, they will not be killed if matched
    #[serde(deserialize_with = "deserialize_regex")]
    pub whitelist: Vec<Regex>,
    // The detail configuration of strategies
    pub strategy: Strategy,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KillTabStrategy {
    /// Kill the tab if all tabs total resident set size (physical memory usage) hit limit, kill in descending order
    RssLimit,
    /// Kill the tab if it is in background for too long, this will not kill "New Tab"
    BackgroundTimeLimit,
    /// Kill the tab if it have not use cpu for too long
    CpuIdleTimeLimit,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Strategy {
    pub rss_limit: RssLimit,
    pub background_time_limit: BackgroundTimeLimit,
    pub cpu_idle_time_limit: CpuIdleTimeLimit,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct RssLimit {
    pub max_bytes: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct BackgroundTimeLimit {
    pub max_secs: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct CpuIdleTimeLimit {
    pub max_secs: f64,
}

pub fn create_or_new_config() -> Config {
    let config_dir = dirs::config_dir().unwrap();
    let config_path = config_dir.join(format!("{PROJECT_NAME}.toml"));
    let config = read_config(&config_path);

    let config = match config {
        Some(config) => config,
        None => overwrite_config_to_default(&config_path),
    };

    config
}

fn read_config(config_path: &PathBuf) -> Option<Config> {
    let config_string = std::fs::read_to_string(config_path.clone());
    match config_string {
        Ok(config_string) => match toml::from_str::<Config>(&config_string) {
            Ok(config) => Some(config),
            Err(e) => {
                eprintln!("The config {:?} have wrong format: {}", config_path, e);
                None
            }
        },
        Err(e) => {
            eprintln!("Couldn't read config from {:?}: {}", config_path, e);
            None
        }
    }
}

fn overwrite_config_to_default(overwridden_config_path: &PathBuf) -> Config {
    let default_config_string = include_str!("config.toml");

    match toml::from_str::<Config>(default_config_string) {
        Ok(config) => {
            match std::fs::write(overwridden_config_path.clone(), default_config_string) {
                Ok(_) => println!("Overwrite config {:?}", overwridden_config_path),
                Err(e) => {
                    eprintln!(
                        "Failed to overwrite config {:?}: {}",
                        overwridden_config_path, e
                    )
                }
            }
            config
        }
        Err(e) => panic!("The default config have incorrect format: {}", e),
    }
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Vec<Regex>, D::Error>
where
    D: Deserializer<'de>,
{
    let regex_strings: Vec<String> = Deserialize::deserialize(deserializer)?;

    regex_strings
        .into_iter()
        .map(|s| Regex::new(&s).map_err(serde::de::Error::custom))
        .collect()
}
