use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn to_merged_json(&self) -> serde_json::Value {
        let mut base = serde_json::json!({
            "id": self.id,
            "provider": self.provider,
            "authType": self.auth_type,
            "name": self.name,
            "email": self.email,
            "priority": self.priority.unwrap_or(1),
            "isActive": self.is_active,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
        });
        if let serde_json::Value::Object(map) = &self.data {
            if let serde_json::Value::Object(ref mut base_map) = base {
                for (k, v) in map {
                    base_map.insert(k.clone(), v.clone());
                }
            }
        }
        base
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
