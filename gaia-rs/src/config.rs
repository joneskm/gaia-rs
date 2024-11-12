use gears::config::ApplicationConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct AppConfig {
    pub example: u32,
}

impl ApplicationConfig for AppConfig {}
