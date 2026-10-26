use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct CcrStore {
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl CcrStore {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn store(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());
        let key = format!("headroom:ccr:{}", hash);

        if let Ok(mut lock) = self.cache.lock() {
            if lock.len() > 1000 {
                // Simple prune if exceeds capacity
                lock.clear();
            }
            lock.insert(key.clone(), content.to_string());
        }

        key
    }

    pub fn retrieve(&self, key: &str) -> Option<String> {
        let lock = self.cache.lock().ok()?;
        lock.get(key).cloned()
    }
}
