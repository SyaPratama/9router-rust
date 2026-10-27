use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub timestamp: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "connectionId")]
    pub connection_id: Option<String>,
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    #[serde(rename = "promptTokens")]
    pub prompt_tokens: Option<i64>,
    #[serde(rename = "completionTokens")]
    pub completion_tokens: Option<i64>,
    pub cost: Option<f64>,
    pub status: Option<String>,
    pub tokens: Option<String>,
    pub meta: Option<String>,
}
fn add_i64(total: &mut Option<i64>, value: Option<i64>) {
    match (total.as_mut(), value) {
        (Some(total), Some(value)) => *total += value,
        _ => *total = None,
    }
}

fn add_f64(total: &mut Option<f64>, value: Option<f64>) {
    match (total.as_mut(), value) {
        (Some(total), Some(value)) => *total += value,
        _ => *total = None,
    }
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
        Self::query_history(conn, None, limit)
    }

    pub fn analytics(
        conn: &Connection,
        period: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<serde_json::Value> {
        use std::collections::BTreeMap;

        let cutoff = match period {
            "today" => Some(now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()),
            "24h" => Some(now - chrono::Duration::hours(24)),
            "7d" => Some(now - chrono::Duration::days(7)),
            "30d" => Some(now - chrono::Duration::days(30)),
            "60d" => Some(now - chrono::Duration::days(60)),
            "all" => None,
            _ => None,
        };
        let cutoff = cutoff.map(|value| value.to_rfc3339());
        let records = Self::query_history(conn, cutoff.as_deref(), usize::MAX)?;
        let mut total_prompt = Some(0_i64);
        let mut total_completion = Some(0_i64);
        let mut total_cost = Some(0.0_f64);
        let mut providers: BTreeMap<String, (i64, Option<i64>, Option<i64>, Option<f64>)> = BTreeMap::new();
        let mut models: BTreeMap<(String, String), (i64, Option<i64>, Option<i64>, Option<f64>)> = BTreeMap::new();
        let mut buckets: BTreeMap<String, (i64, Option<i64>, Option<f64>)> = BTreeMap::new();
        let mut logs = Vec::with_capacity(records.len());

        for record in &records {
            add_i64(&mut total_prompt, record.prompt_tokens);
            add_i64(&mut total_completion, record.completion_tokens);
            add_f64(&mut total_cost, record.cost);
            let provider = record.provider.as_deref().unwrap_or("unknown");
            let model = record.model.as_deref().unwrap_or("unknown");
            let provider_totals = providers.entry(provider.to_owned()).or_insert((0, Some(0), Some(0), Some(0.0)));
            provider_totals.0 += 1;
            add_i64(&mut provider_totals.1, record.prompt_tokens);
            add_i64(&mut provider_totals.2, record.completion_tokens);
            add_f64(&mut provider_totals.3, record.cost);
            let model_totals = models.entry((provider.to_owned(), model.to_owned())).or_insert((0, Some(0), Some(0), Some(0.0)));
            model_totals.0 += 1;
            add_i64(&mut model_totals.1, record.prompt_tokens);
            add_i64(&mut model_totals.2, record.completion_tokens);
            add_f64(&mut model_totals.3, record.cost);

            if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&record.timestamp) {
                let timestamp = timestamp.with_timezone(&chrono::Utc);
                let bucket = if matches!(period, "today" | "24h") {
                    timestamp.format("%Y-%m-%dT%H:00:00Z").to_string()
                } else {
                    timestamp.format("%Y-%m-%dT00:00:00Z").to_string()
                };
                let bucket_totals = buckets.entry(bucket).or_insert((0, Some(0), Some(0.0)));
                bucket_totals.0 += 1;
                let tokens = record.prompt_tokens.zip(record.completion_tokens).map(|(input, output)| input + output);
                add_i64(&mut bucket_totals.1, tokens);
                add_f64(&mut bucket_totals.2, record.cost);
            }
            logs.push(format!(
                "{} | {} | {} | {} | {} | {}",
                record.timestamp,
                model,
                provider,
                record.prompt_tokens.map(|value| value.to_string()).unwrap_or_else(|| "unavailable".to_string()),
                record.completion_tokens.map(|value| value.to_string()).unwrap_or_else(|| "unavailable".to_string()),
                record.status.as_deref().unwrap_or("unknown")
            ));
        }

        let breakdown = models.into_iter().map(|((provider, model), (requests, input, output, cost))| {
            serde_json::json!({
                "model": model,
                "provider": provider,
                "requests": requests,
                "inputTokens": input,
                "outputTokens": output,
                "totalTokens": input.zip(output).map(|(input, output)| input + output),
                "cost": cost
            })
        }).collect::<Vec<_>>();
        let by_provider = providers.into_iter().map(|(provider, (requests, input, output, cost))| {
            (provider, serde_json::json!({
                "requests": requests,
                "promptTokens": input,
                "completionTokens": output,
                "cost": cost
            }))
        }).collect::<serde_json::Map<_, _>>();
        let chart = buckets.into_iter().map(|(timestamp, (requests, tokens, cost))| {
            serde_json::json!({ "timestamp": timestamp, "requests": requests, "tokens": tokens, "cost": cost })
        }).collect::<Vec<_>>();

        Ok(serde_json::json!({
            "stats": {
                "totalRequests": records.len(),
                "totalPromptTokens": total_prompt,
                "totalCompletionTokens": total_completion,
                "totalTokens": total_prompt.zip(total_completion).map(|(input, output)| input + output),
                "totalCost": total_cost,
                "byProvider": by_provider,
                "period": period
            },
            "history": records,
            "breakdown": breakdown,
            "chart": chart,
            "logs": logs
        }))
    }

    fn query_history(conn: &Connection, cutoff: Option<&str>, limit: usize) -> Result<Vec<UsageRecord>> {
        let mut stmt = conn.prepare(
            "SELECT timestamp, provider, model, connectionId, apiKey, endpoint,
                    promptTokens, completionTokens, cost, status, tokens, meta
             FROM usageHistory
             WHERE (?1 IS NULL OR julianday(timestamp) >= julianday(?1))
             ORDER BY id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![cutoff, limit.min(i64::MAX as usize) as i64], |row| {
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
        rows.collect()
    }

    pub fn stats(conn: &Connection) -> Result<serde_json::Value> {
        Ok(Self::analytics(conn, "all", chrono::Utc::now())?["stats"].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE usageHistory (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, provider TEXT, model TEXT, connectionId TEXT, apiKey TEXT, endpoint TEXT, promptTokens INTEGER, completionTokens INTEGER, cost REAL, status TEXT, tokens TEXT, meta TEXT);").unwrap();
        conn
    }

    fn record(timestamp: &str, provider: &str, model: &str, input: i64, output: i64, cost: f64) -> UsageRecord {
        UsageRecord {
            timestamp: timestamp.into(), provider: Some(provider.into()), model: Some(model.into()),
            connection_id: None, api_key: Some("secret".into()), endpoint: None,
            prompt_tokens: Some(input), completion_tokens: Some(output), cost: Some(cost), status: Some("success".into()),
            tokens: None, meta: None,
        }
    }

    #[test]
    fn filters_periods_and_aggregates_persisted_requests() {
        let conn = db();
        let now = chrono::DateTime::parse_from_rfc3339("2025-04-10T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        UsageRepo::record(&conn, &record("2025-04-10T11:00:00Z", "p1", "m1", 10, 2, 0.5)).unwrap();
        UsageRepo::record(&conn, &record("2025-04-09T12:00:00Z", "p2", "m2", 20, 3, 1.25)).unwrap();
        UsageRepo::record(&conn, &record("2025-04-09T11:59:59Z", "p3", "m3", 99, 9, 9.0)).unwrap();
        let data = UsageRepo::analytics(&conn, "24h", now).unwrap();
        assert_eq!(data["stats"]["totalRequests"], 2);
        assert_eq!(data["stats"]["totalPromptTokens"], 30);
        assert_eq!(data["stats"]["totalCompletionTokens"], 5);
        assert_eq!(data["stats"]["totalCost"], 1.75);
        assert_eq!(data["breakdown"].as_array().unwrap().len(), 2);
        assert_eq!(data["chart"].as_array().unwrap().len(), 2);
        assert_eq!(data["logs"].as_array().unwrap().len(), 2);
        assert_eq!(data["history"].as_array().unwrap().len(), 2);
        assert_eq!(data["breakdown"][0]["model"], "m1");
        assert_eq!(data["chart"][0]["tokens"], 23);
        assert!(data["logs"].as_array().unwrap().iter().any(|log| log.as_str().unwrap().contains("| p1 |")));
        assert_eq!(data["history"][0]["model"], "m2");
        assert!(!serde_json::to_string(&data).unwrap().contains("m3"));
        assert!(!serde_json::to_string(&data).unwrap().contains("secret"));
    }

    #[test]
    fn untracked_metrics_remain_unavailable() {
        let conn = db();
        let mut request = record("2025-04-10T11:00:00Z", "provider", "model", 0, 0, 0.0);
        request.prompt_tokens = None;
        request.completion_tokens = None;
        request.cost = None;
        UsageRepo::record(&conn, &request).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2025-04-10T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        let data = UsageRepo::analytics(&conn, "today", now).unwrap();
        assert!(data["stats"]["totalPromptTokens"].is_null());
        assert!(data["stats"]["totalCompletionTokens"].is_null());
        assert!(data["stats"]["totalCost"].is_null());
        assert!(data["breakdown"][0]["totalTokens"].is_null());
        assert!(data["chart"][0]["tokens"].is_null());
        assert!(data["logs"][0].as_str().unwrap().contains("unavailable"));
    }

    #[test]
    fn all_supported_periods_include_their_exact_boundary() {
        let conn = db();
        let now = chrono::DateTime::parse_from_rfc3339("2025-04-10T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        for timestamp in [
            "2025-02-08T12:00:00Z",
            "2025-02-09T12:00:00Z",
            "2025-03-11T12:00:00Z",
            "2025-04-03T12:00:00Z",
            "2025-04-09T12:00:00Z",
            "2025-04-10T11:00:00Z",
        ] {
            UsageRepo::record(&conn, &record(timestamp, "provider", "model", 1, 1, 1.0)).unwrap();
        }
        for (period, expected) in [("today", 1), ("24h", 2), ("7d", 3), ("30d", 4), ("60d", 5), ("all", 6)] {
            let analytics = UsageRepo::analytics(&conn, period, now).unwrap();
            assert_eq!(analytics["stats"]["totalRequests"], expected, "period {period}");
        }
    }

    #[test]
    fn empty_period_has_zero_totals_and_empty_views() {
        let conn = db();
        let now = chrono::Utc::now();
        let data = UsageRepo::analytics(&conn, "today", now).unwrap();
        assert_eq!(data["stats"]["totalRequests"], 0);
        assert_eq!(data["stats"]["totalTokens"], 0);
        assert_eq!(data["stats"]["totalCost"], 0.0);
        for key in ["history", "breakdown", "chart", "logs"] {
            assert!(data[key].as_array().unwrap().is_empty());
        }
    }
}
