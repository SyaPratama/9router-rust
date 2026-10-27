use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPoolItem {
    pub id: String,
    #[serde(rename = "isActive", default = "default_true")]
    pub is_active: bool,
    #[serde(rename = "testStatus")]
    pub test_status: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(rename = "createdAt", default = "default_now")]
    pub created_at: String,
    #[serde(rename = "updatedAt", default = "default_now")]
    pub updated_at: String,
}

pub struct ProxyPoolsRepo;

impl ProxyPoolsRepo {
    pub fn list_all(conn: &Connection) -> Result<Vec<ProxyPoolItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, isActive, testStatus, data, createdAt, updatedAt FROM proxyPools ORDER BY createdAt DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            let data_str: String = row.get(3)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null);
            let is_active_int: i32 = row.get(1)?;
            Ok(ProxyPoolItem {
                id: row.get(0)?,
                is_active: is_active_int == 1,
                test_status: row.get(2)?,
                data,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<ProxyPoolItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, isActive, testStatus, data, createdAt, updatedAt FROM proxyPools WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let data_str: String = row.get(3)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null);
            let is_active_int: i32 = row.get(1)?;
            Ok(Some(ProxyPoolItem {
                id: row.get(0)?,
                is_active: is_active_int == 1,
                test_status: row.get(2)?,
                data,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn upsert(conn: &Connection, item: &ProxyPoolItem) -> Result<()> {
        let data_str = serde_json::to_string(&item.data).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO proxyPools (id, isActive, testStatus, data, createdAt, updatedAt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                isActive = ?2,
                testStatus = ?3,
                data = ?4,
                updatedAt = ?6",
            params![
                item.id,
                if item.is_active { 1 } else { 0 },
                item.test_status,
                data_str,
                item.created_at,
                item.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let count = conn.execute("DELETE FROM proxyPools WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }

    pub fn update_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE proxyPools SET testStatus = ?1, updatedAt = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;
        Ok(())
    }
}
