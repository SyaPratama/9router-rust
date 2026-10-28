use rusqlite::{params, Connection, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConnection {
    pub id: String,
    pub provider: String,
    #[serde(rename = "authType")]
    pub auth_type: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub priority: Option<i64>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl ProviderConnection {
    pub fn is_sensitive_key(key: &str) -> bool {
        let normalized: String = key.chars().filter(|ch| ch.is_ascii_alphanumeric()).flat_map(char::to_lowercase).collect();
        normalized.contains("token") || normalized.contains("secret") || normalized.contains("authorization") || normalized.contains("apikey") || normalized.contains("code") || normalized == "key" || normalized.ends_with("key")
    }

    pub fn to_merged_json(&self) -> serde_json::Value {
        let mut base = serde_json::json!({
            "id": self.id, "provider": self.provider, "authType": self.auth_type,
            "name": self.name, "email": self.email, "priority": self.priority.unwrap_or(1),
            "isActive": self.is_active, "createdAt": self.created_at, "updatedAt": self.updated_at,
        });
        if let (serde_json::Value::Object(map), serde_json::Value::Object(base_map)) = (&self.data, &mut base) {
            for (key, value) in map {
                if !Self::is_sensitive_key(key) {
                    base_map.insert(key.clone(), redact_credentials(value));
                }
            }
            let api_key = map.iter().find(|(key, _)| Self::is_sensitive_key(key) && key.to_ascii_lowercase().contains("apikey"));
            let access_token = map.iter().find(|(key, _)| key.to_ascii_lowercase().contains("accesstoken") || key.to_ascii_lowercase().contains("access_token"));
            base_map.insert("hasApiKey".into(), serde_json::json!(api_key.is_some_and(|(_, value)| !value.as_str().unwrap_or_default().is_empty())));
            base_map.insert("hasOAuthToken".into(), serde_json::json!(access_token.is_some_and(|(_, value)| !value.as_str().unwrap_or_default().is_empty())));
        }
        base
    }
}

fn redact_credentials(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(map.iter()
            .filter(|(key, _)| !ProviderConnection::is_sensitive_key(key))
            .map(|(key, value)| (key.clone(), redact_credentials(value))).collect()),
        serde_json::Value::Array(values) => serde_json::Value::Array(values.iter().map(redact_credentials).collect()),
        value => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderConnection;

    #[test]
    fn merged_json_never_returns_credentials() {
        let connection = ProviderConnection {
            id: "conn".into(), provider: "antigravity".into(), auth_type: "oauth".into(),
            name: None, email: None, priority: Some(1), is_active: true,
            data: serde_json::json!({
                "apiKey": "key-secret", "accessToken": "access-secret", "access_token": "snake-access-secret",
                "refreshToken": "refresh-secret", "idToken": "id-secret", "clientSecret": "client-secret",
                "code": "auth-code", "baseUrl": "https://example.test",
                "providerSpecificData": { "refresh_token": "nested-refresh-secret" }
            }),
            created_at: "now".into(), updated_at: "now".into(),
        };
        let json = connection.to_merged_json().to_string();
        for secret in ["key-secret", "access-secret", "snake-access-secret", "refresh-secret", "id-secret", "client-secret", "auth-code", "nested-refresh-secret"] {
            assert!(!json.contains(secret));
        }
        assert!(json.contains("https://example.test"));
    }
}

pub struct ConnectionsRepo;

impl ConnectionsRepo {
    pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<ProviderConnection>> {
        let mut stmt = conn.prepare(
            "SELECT id, provider, authType, name, email, priority, isActive, data, createdAt, updatedAt
             FROM providerConnections
             WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let data_str: String = row.get(7)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null);
            let is_active_int: i64 = row.get(6)?;
            Ok(Some(ProviderConnection {
                id: row.get(0)?,
                provider: row.get(1)?,
                auth_type: row.get(2)?,
                name: row.get(3)?,
                email: row.get(4)?,
                priority: row.get(5)?,
                is_active: is_active_int == 1,
                data,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_by_provider(conn: &Connection, provider: &str) -> Result<Vec<ProviderConnection>> {
        let mut stmt = conn.prepare(
            "SELECT id, provider, authType, name, email, priority, isActive, data, createdAt, updatedAt
             FROM providerConnections
             WHERE provider = ?1
             ORDER BY priority ASC, createdAt ASC",
        )?;
        let rows = stmt.query_map(params![provider], |row| {
            let data_str: String = row.get(7)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null);
            let is_active_int: i64 = row.get(6)?;
            Ok(ProviderConnection {
                id: row.get(0)?,
                provider: row.get(1)?,
                auth_type: row.get(2)?,
                name: row.get(3)?,
                email: row.get(4)?,
                priority: row.get(5)?,
                is_active: is_active_int == 1,
                data,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn list_active_by_provider(conn: &Connection, provider: &str) -> Result<Vec<ProviderConnection>> {
        let mut stmt = conn.prepare(
            "SELECT id, provider, authType, name, email, priority, isActive, data, createdAt, updatedAt
             FROM providerConnections
             WHERE provider = ?1 AND isActive = 1
             ORDER BY priority ASC, createdAt ASC",
        )?;

        let rows = stmt.query_map(params![provider], |row| {
            let data_str: String = row.get(7)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null);
            let is_active_int: i64 = row.get(6)?;
            Ok(ProviderConnection {
                id: row.get(0)?,
                provider: row.get(1)?,
                auth_type: row.get(2)?,
                name: row.get(3)?,
                email: row.get(4)?,
                priority: row.get(5)?,
                is_active: is_active_int == 1,
                data,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<ProviderConnection>> {
        let mut stmt = conn.prepare(
            "SELECT id, provider, authType, name, email, priority, isActive, data, createdAt, updatedAt
             FROM providerConnections
             ORDER BY provider ASC, priority ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let data_str: String = row.get(7)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null);
            let is_active_int: i64 = row.get(6)?;
            Ok(ProviderConnection {
                id: row.get(0)?,
                provider: row.get(1)?,
                auth_type: row.get(2)?,
                name: row.get(3)?,
                email: row.get(4)?,
                priority: row.get(5)?,
                is_active: is_active_int == 1,
                data,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn upsert(conn: &Connection, item: &ProviderConnection) -> Result<()> {
        let data_str = serde_json::to_string(&item.data).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO providerConnections (id, provider, authType, name, email, priority, isActive, data, createdAt, updatedAt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                provider = ?2,
                authType = ?3,
                name = ?4,
                email = ?5,
                priority = ?6,
                isActive = ?7,
                data = ?8,
                updatedAt = ?10",
            params![
                item.id,
                item.provider,
                item.auth_type,
                item.name,
                item.email,
                item.priority,
                if item.is_active { 1 } else { 0 },
                data_str,
                item.created_at,
                item.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let count = conn.execute("DELETE FROM providerConnections WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }
}
