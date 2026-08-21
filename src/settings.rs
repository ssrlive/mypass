use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const MAX_RECENT_FILES: usize = 10;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProxyProtocol {
    #[default]
    None,
    Http,
    Socks5,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProxySettings {
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_position: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_size: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_tree_panel: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_files: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxySettings>,
}

impl Settings {
    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|path| path.join("mypass").join("settings.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        let Ok(data) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&data).unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        let Some(parent) = path.parent() else {
            return;
        };
        if let Err(error) = fs::create_dir_all(parent) {
            log::warn!("Could not create settings directory: {error}");
            return;
        }
        match serde_json::to_string_pretty(self) {
            Ok(data) => {
                if let Err(error) = fs::write(path, data) {
                    log::warn!("Could not save settings: {error}");
                }
            }
            Err(error) => log::warn!("Could not serialize settings: {error}"),
        }
    }

    pub fn add_recent_file(&mut self, path: impl Into<String>) {
        let path = path.into();
        let recent_files = self.recent_files.get_or_insert_with(Vec::new);
        recent_files.retain(|recent| recent != &path);
        recent_files.insert(0, path);
        recent_files.truncate(MAX_RECENT_FILES);
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_RECENT_FILES, Settings};

    #[test]
    fn recent_files_are_unique_and_limited() {
        let mut settings = Settings::default();
        for index in 0..=MAX_RECENT_FILES {
            settings.add_recent_file(format!("file-{index}"));
        }
        settings.add_recent_file("file-5");

        let recent_files = settings.recent_files.as_ref().expect("recent files should exist");
        assert_eq!(recent_files.len(), MAX_RECENT_FILES);
        assert_eq!(recent_files[0], "file-5");
        assert!(!recent_files.contains(&"file-0".to_string()));
    }
}
