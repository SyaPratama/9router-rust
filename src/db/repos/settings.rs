use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSettings {
    #[serde(default = "default_true", alias = "headroom_enabled")]
    pub headroom_enabled: bool,
    #[serde(default = "default_true", alias = "rtk_enabled")]
    pub rtk_enabled: bool,
    #[serde(default, alias = "caveman_level")]
    pub caveman_level: Option<String>,
    #[serde(default, alias = "ponytail_level")]
    pub ponytail_level: Option<String>,
    #[serde(default, alias = "require_api_key")]
    pub require_api_key: bool,
    #[serde(default = "default_true", alias = "require_login")]
    pub require_login: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}
fn default_port() -> u16 {
    20128
}

impl Default for SystemSettings {
    fn default() -> Self {
        let mut extra = serde_json::Map::new();
        extra.insert("cloudEnabled".to_string(), serde_json::json!(false));
        extra.insert("tunnelEnabled".to_string(), serde_json::json!(false));
        extra.insert("tunnelUrl".to_string(), serde_json::json!(""));
        extra.insert("tunnelProvider".to_string(), serde_json::json!("cloudflare"));
        extra.insert("tailscaleEnabled".to_string(), serde_json::json!(false));
        extra.insert("tailscaleUrl".to_string(), serde_json::json!(""));
        extra.insert("stickyRoundRobinLimit".to_string(), serde_json::json!(3));
        extra.insert("providerStrategies".to_string(), serde_json::json!({}));
        extra.insert("quotaVisibility".to_string(), serde_json::json!({}));
        extra.insert("comboStrategy".to_string(), serde_json::json!("fallback"));
        extra.insert("comboStickyRoundRobinLimit".to_string(), serde_json::json!(1));
        extra.insert("comboStrategies".to_string(), serde_json::json!({}));
        extra.insert("capacityAdapter".to_string(), serde_json::json!({
            "vision": { "enabled": true, "roundRobin": false, "models": [] },
            "pdf": { "enabled": false, "roundRobin": false, "models": [] },
            "audioInput": { "enabled": true, "roundRobin": false, "models": [] },
            "videoInput": { "enabled": false, "roundRobin": false, "models": [] }
        }));
        extra.insert("tunnelDashboardAccess".to_string(), serde_json::json!(true));
        extra.insert("authMode".to_string(), serde_json::json!("password"));
        extra.insert("ssoType".to_string(), serde_json::json!("oidc"));

        Self {
            headroom_enabled: true,
            rtk_enabled: true,
            caveman_level: None,
            ponytail_level: None,
            require_api_key: false,
            require_login: true,
            port: 20128,
            extra,
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
