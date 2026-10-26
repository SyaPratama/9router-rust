use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    #[serde(default = "default_true")]
    pub headroom_enabled: bool,
    #[serde(default = "default_true")]
    pub rtk_enabled: bool,
    #[serde(default)]
    pub caveman_level: Option<String>,
    #[serde(default)]
    pub ponytail_level: Option<String>,
    #[serde(default)]
    pub require_api_key: bool,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_true() -> bool { true }
fn default_port() -> u16 { 20128 }

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            headroom_enabled: true,
            rtk_enabled: true,
            caveman_level: None,
            ponytail_level: None,
            require_api_key: false,
            port: 20128,
        }
    }
}

pub struct SettingsRepo;

impl SettingsRepo {
    pub fn get(conn: &Connection) -> Result<SystemSettings> {
        let mut stmt = conn.prepare("SELECT data FROM settings WHERE id = 1")?;
        let res = stmt.query_row([], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        });

        match res {
            Ok(json_str) => {
                let s = serde_json::from_str(&json_str).unwrap_or_default();
                Ok(s)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let default_settings = SystemSettings::default();
                let json = serde_json::to_string(&default_settings).unwrap_or_else(|_| "{}".to_string());
                let _ = conn.execute(
                    "INSERT INTO settings (id, data) VALUES (1, ?1)",
                    params![json],
                );
                Ok(default_settings)
            }
            Err(e) => Err(e),
        }
    }

    pub fn save(conn: &Connection, settings: &SystemSettings) -> Result<()> {
        let json = serde_json::to_string(settings).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO settings (id, data) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET data = ?1",
            params![json],
        )?;
        Ok(())
    }
}
