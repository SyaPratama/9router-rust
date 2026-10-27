use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub timestamp: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "connectionId")]
    pub connection_id: Option<String>,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    #[serde(rename = "promptTokens")]
    pub prompt_tokens: i64,
    #[serde(rename = "completionTokens")]
    pub completion_tokens: i64,
    pub cost: f64,
    pub status: Option<String>,
    pub tokens: Option<String>,
    pub meta: Option<String>,
}

pub struct UsageRepo;

impl UsageRepo {
    pub fn record(conn: &Connection, record: &UsageRecord) -> Result<()> {
        conn.execute(
            "INSERT INTO usageHistory (
                timestamp, provider, model, connectionId, apiKey, endpoint,
                promptTokens, completionTokens, cost, status, tokens, meta
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.timestamp,
                record.provider,
                record.model,
                record.connection_id,
                record.api_key,
                record.endpoint,
                record.prompt_tokens,
                record.completion_tokens,
                record.cost,
                record.status,
                record.tokens,
                record.meta,
            ],
        )?;
        Ok(())
    }

    pub fn history(conn: &Connection, limit: usize) -> Result<Vec<UsageRecord>> {
        let mut stmt = conn.prepare(
            "SELECT timestamp, provider, model, connectionId, apiKey, endpoint,
                    promptTokens, completionTokens, cost, status, tokens, meta
             FROM usageHistory
             ORDER BY id DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(UsageRecord {
                timestamp: row.get(0)?,
                provider: row.get(1)?,
                model: row.get(2)?,
                connection_id: row.get(3)?,
                api_key: row.get(4)?,
                endpoint: row.get(5)?,
                prompt_tokens: row.get(6)?,
                completion_tokens: row.get(7)?,
                cost: row.get(8)?,
                status: row.get(9)?,
                tokens: row.get(10)?,
                meta: row.get(11)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn stats(conn: &Connection) -> Result<serde_json::Value> {
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), SUM(promptTokens), SUM(completionTokens), SUM(cost) FROM usageHistory",
        )?;

        let row = stmt.query_row([], |r| {
            let count: i64 = r.get(0)?;
            let prompt: i64 = r.get(1).unwrap_or(0);
            let completion: i64 = r.get(2).unwrap_or(0);
            let cost: f64 = r.get(3).unwrap_or(0.0);
            Ok((count, prompt, completion, cost))
        })?;

        Ok(serde_json::json!({
            "totalRequests": row.0,
            "totalPromptTokens": row.1,
            "totalCompletionTokens": row.2,
            "totalTokens": row.1 + row.2,
            "totalCost": row.3
        }))
    }
}
