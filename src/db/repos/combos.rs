use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCombo {
    pub id: String,
    pub name: String,
    pub kind: Option<String>,
    pub models: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

pub struct CombosRepo;

impl CombosRepo {
    pub fn get_by_name(conn: &Connection, name: &str) -> Result<Option<ModelCombo>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, models, createdAt, updatedAt FROM combos WHERE name = ?1"
        )?;

        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            let models_str: String = row.get(3)?;
            let models: Vec<String> = serde_json::from_str(&models_str).unwrap_or_default();
            Ok(Some(ModelCombo {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                models,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<ModelCombo>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, models, createdAt, updatedAt FROM combos WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let models: Vec<String> = serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default();
            Ok(Some(ModelCombo {
                id: row.get(0)?, name: row.get(1)?, kind: row.get(2)?, models,
                created_at: row.get(4)?, updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<ModelCombo>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, models, createdAt, updatedAt FROM combos ORDER BY name ASC"
        )?;

        let rows = stmt.query_map([], |row| {
            let models_str: String = row.get(3)?;
            let models: Vec<String> = serde_json::from_str(&models_str).unwrap_or_default();
            Ok(ModelCombo {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                models,
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

    pub fn upsert(conn: &Connection, item: &ModelCombo) -> Result<()> {
        let models_str = serde_json::to_string(&item.models).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO combos (id, name, kind, models, createdAt, updatedAt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(name) DO UPDATE SET
                kind = ?3,
                models = ?4,
                updatedAt = ?6",
            params![
                item.id,
                item.name,
                item.kind,
                models_str,
                item.created_at,
                item.updated_at,
            ],
        )?;
        Ok(())
    }
    pub fn insert(conn: &Connection, item: &ModelCombo) -> Result<()> {
        let models = serde_json::to_string(&item.models).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO combos (id, name, kind, models, createdAt, updatedAt) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![item.id, item.name, item.kind, models, item.created_at, item.updated_at],
        )?;
        Ok(())
    }
    pub fn update(conn: &Connection, item: &ModelCombo) -> Result<bool> {
        let models = serde_json::to_string(&item.models).unwrap_or_else(|_| "[]".to_string());
        let changed = conn.execute(
            "UPDATE combos SET name = ?2, kind = ?3, models = ?4, updatedAt = ?5 WHERE id = ?1",
            params![item.id, item.name, item.kind, models, item.updated_at],
        )?;
        Ok(changed > 0)
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let count = conn.execute("DELETE FROM combos WHERE id = ?1 OR name = ?1", params![id])?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::{CombosRepo, ModelCombo};
    use rusqlite::Connection;

    #[test]
    fn create_and_update_preserve_identity_and_creation_time() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE combos (id TEXT PRIMARY KEY, name TEXT UNIQUE, kind TEXT, models TEXT, createdAt TEXT, updatedAt TEXT);").unwrap();
        let combo = ModelCombo {
            id: "id".into(), name: "before".into(), kind: None, models: vec!["cu/default".into()],
            created_at: "created".into(), updated_at: "created".into(),
        };
        CombosRepo::insert(&conn, &combo).unwrap();
        let updated = ModelCombo { name: "after".into(), models: vec!["cc/opus".into()], updated_at: "updated".into(), ..combo };
        assert!(CombosRepo::update(&conn, &updated).unwrap());
        let saved = CombosRepo::get_by_id(&conn, "id").unwrap().unwrap();
        assert_eq!(saved.name, "after");
        assert_eq!(saved.models[0], "cc/opus");
        assert_eq!(saved.created_at, "created");
        assert_eq!(saved.updated_at, "updated");
        assert!(!CombosRepo::update(&conn, &ModelCombo { id: "missing".into(), ..updated }).unwrap());
    }
}
