use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyItem {
    pub id: String,
    pub key: String,
    pub name: Option<String>,
    #[serde(rename = "machineId")]
    pub machine_id: Option<String>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "accessRestricted")]
    pub access_restricted: bool,
    #[serde(rename = "accessAllow")]
    pub access_allow: Option<String>,
}

pub struct ApiKeysRepo;

impl ApiKeysRepo {
    pub fn find_by_key(conn: &Connection, key: &str) -> Result<Option<ApiKeyItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, key, name, machineId, isActive, createdAt, accessRestricted, accessAllow
             FROM apiKeys WHERE key = ?1"
        )?;

        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let active_int: i64 = row.get(4)?;
            let restricted_int: i64 = row.get(6)?;
            Ok(Some(ApiKeyItem {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                machine_id: row.get(3)?,
                is_active: active_int == 1,
                created_at: row.get(5)?,
                access_restricted: restricted_int == 1,
                access_allow: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<ApiKeyItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, key, name, machineId, isActive, createdAt, accessRestricted, accessAllow
             FROM apiKeys ORDER BY createdAt DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            let active_int: i64 = row.get(4)?;
            let restricted_int: i64 = row.get(6)?;
            Ok(ApiKeyItem {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                machine_id: row.get(3)?,
                is_active: active_int == 1,
                created_at: row.get(5)?,
                access_restricted: restricted_int == 1,
                access_allow: row.get(7)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn insert(conn: &Connection, item: &ApiKeyItem) -> Result<()> {
        conn.execute(
            "INSERT INTO apiKeys (id, key, name, machineId, isActive, createdAt, accessRestricted, accessAllow)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                item.id,
                item.key,
                item.name,
                item.machine_id,
                if item.is_active { 1 } else { 0 },
                item.created_at,
                if item.access_restricted { 1 } else { 0 },
                item.access_allow,
            ],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let count = conn.execute("DELETE FROM apiKeys WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }
}
