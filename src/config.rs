use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::eq::EqProfile;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub input_dev_name: Option<String>,
    pub output_dev_name: Option<String>,
    pub latency: u32,
    pub eq_profile: EqProfile,
}

pub fn config_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap();
    dir.push("eq_layer");
    dir.push("config.toml");
    dir
}

impl Config {
    pub fn save(&self) -> Result<()> {
        std::fs::write(config_dir(), toml::to_string(&self)?)?;
        Ok(())
    }
}
