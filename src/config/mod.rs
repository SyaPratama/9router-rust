use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub host: String,
    pub data_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(20128);

        let host = std::env::var("HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let data_dir = std::env::var("DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = dirs_home().unwrap_or_else(|| PathBuf::from("."));
                home.join(".9router")
            });

        Self { port, host, data_dir }
    }
}

pub fn dirs_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}
