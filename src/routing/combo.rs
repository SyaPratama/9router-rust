use rusqlite::Connection;
use crate::db::repos::combos::CombosRepo;

pub fn resolve_model_chain(conn: &Connection, requested_model: &str) -> Vec<String> {
    // If it's a known combo name
    if let Ok(Some(combo)) = CombosRepo::get_by_name(conn, requested_model) {
        if !combo.models.is_empty() {
            return combo.models;
        }
    }

    vec![requested_model.to_string()]
}
