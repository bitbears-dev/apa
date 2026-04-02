use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct AppConfig {
    pub openai_api_key: Option<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        let mut config = AppConfig::default();

        // Use directories crate to find ~/.config/apa/config.toml
        if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "apa") {
            let config_file = proj_dirs.config_dir().join("config.toml");
            if let Ok(contents) = std::fs::read_to_string(&config_file)
                && let Ok(parsed) = toml::from_str::<AppConfig>(&contents)
            {
                config.openai_api_key = parsed.openai_api_key;
            }
        }

        // Env var overrides config file
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            config.openai_api_key = Some(key);
        }

        config
    }
}
