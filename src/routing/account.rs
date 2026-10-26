use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::db::repos::connections::{ConnectionsRepo, ProviderConnection};

#[derive(Clone, Default)]
pub struct AccountManager {
    strikes: Arc<Mutex<HashMap<String, u32>>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_available_connections(
        &self,
        conn: &Connection,
        provider: &str,
    ) -> Vec<ProviderConnection> {
        let active = ConnectionsRepo::list_active_by_provider(conn, provider).unwrap_or_default();
        let strikes = self.strikes.lock().unwrap_or_else(|e| e.into_inner());

        // Sort by strikes (lowest strikes first), then priority
        let mut sorted = active;
        sorted.sort_by_key(|c| {
            let strike_count = strikes.get(&c.id).copied().unwrap_or(0);
            (strike_count, c.priority.unwrap_or(100))
        });

        sorted
    }

    pub fn record_strike(&self, connection_id: &str) {
        if let Ok(mut lock) = self.strikes.lock() {
            let count = lock.entry(connection_id.to_string()).or_insert(0);
            *count += 1;
        }
    }

    pub fn clear_strikes(&self, connection_id: &str) {
        if let Ok(mut lock) = self.strikes.lock() {
            lock.remove(connection_id);
        }
    }
}
