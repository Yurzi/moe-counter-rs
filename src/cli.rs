use clap::Parser;
use clap::{arg, command};
use serde::{Deserialize, Serialize};

const APP_NAME: &'static str = "moe-counter";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sqlite {
    pub path: String,
    pub table_name: String,
}

impl Default for Sqlite {
    fn default() -> Self {
        Sqlite {
            path: "data.db".to_string(),
            table_name: "count".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub listen: String,
    pub port: u16,
    pub themes_dir: String,
    pub default_theme: String,
    pub digit_count: u32,
    pub default_format: String,
    pub pixelated: bool,
    pub sqlite: Sqlite,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen: "127.0.0.1".to_string(),
            port: 9534,
            themes_dir: "themes".to_string(),
            default_theme: "moebooru".to_string(),
            digit_count: 0,
            default_format: "svg".to_string(),
            pixelated: false,
            sqlite: Sqlite::default(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[arg(
        short,
        long,
        help = "path to config file",
        default_value = "moe-counter-rs.toml"
    )]
    pub config_path: String,
}

pub fn read_config(config_path: &str) -> Config {
    // check config file is exist
    if !std::path::Path::new(config_path)
        .try_exists()
        .expect("hit error when check config file")
    {
        // if config is not exist.
        // create a default config
        let cfg = Config::default();
        confy::store_path(config_path, cfg.clone())
            .expect(&format!("failed to init config file: {config_path}"));
        return cfg;
    }
    // read config from file
    let cfg =
        confy::load_path(config_path).expect(&format!("failed to load config file: {config_path}"));
    cfg
}
