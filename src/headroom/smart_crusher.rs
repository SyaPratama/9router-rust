use serde_json::Value;

pub struct SmartCrusher;

impl SmartCrusher {
    pub fn compress(input: &str) -> Option<String> {
        let val: Value = serde_json::from_str(input).ok()?;
        
        if let Some(arr) = val.as_array() {
            if arr.is_empty() {
                return None;
            }
            return Self::compress_array(arr);
        }

        if let Some(obj) = val.as_object() {
            // Find inner array to compress
            let mut modified = false;
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                if let Some(arr) = v.as_array() {
                    if let Some(compressed_arr_str) = Self::compress_array(arr) {
                        if let Ok(comp_val) = serde_json::from_str::<Value>(&compressed_arr_str) {
                            new_obj.insert(k.clone(), comp_val);
                            modified = true;
                            continue;
                        }
                    }
                }
                new_obj.insert(k.clone(), Self::compact_value(v));
            }
            if modified {
                return serde_json::to_string(&Value::Object(new_obj)).ok();
            }
        }

        None
    }

    fn compress_array(arr: &[Value]) -> Option<String> {
        if arr.len() < 2 {
            return None;
        }

        // Check if array elements are uniform objects
        let first = arr.first()?.as_object()?;
        let keys: Vec<String> = first.keys().cloned().collect();
        if keys.is_empty() {
            return None;
        }

        let is_uniform = arr.iter().all(|item| {
            if let Some(obj) = item.as_object() {
                keys.iter().all(|k| obj.contains_key(k))
            } else {
                false
            }
        });

        if !is_uniform {
            // Compact values in place
            let compacted: Vec<Value> = arr.iter().map(Self::compact_value).collect();
            return serde_json::to_string(&compacted).ok();
        }

        // Compact into schema + rows format
        let mut rows = Vec::new();
        for item in arr {
            let obj = item.as_object()?;
            let row: Vec<Value> = keys.iter().map(|k| {
                Self::compact_value(obj.get(k).unwrap_or(&Value::Null))
            }).collect();
            rows.push(row);
        }

        let compacted_tabular = serde_json::json!({
            "__schema": keys,
            "__rows": rows
        });

        serde_json::to_string(&compacted_tabular).ok()
    }

    fn compact_value(v: &Value) -> Value {
        match v {
            Value::String(s) => {
                // Keep UUIDs, hashes, numbers, IDs
                if s.len() > 200 && !s.contains('-') && !s.starts_with("http") {
                    let head = &s[..120];
                    let tail = &s[s.len() - 40..];
                    Value::String(format!("{}...[len: {}]...{}", head, s.len(), tail))
                } else {
                    v.clone()
                }
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(Self::compact_value).collect())
            }
            Value::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, val) in obj {
                    map.insert(k.clone(), Self::compact_value(val));
                }
                Value::Object(map)
            }
            _ => v.clone(),
        }
    }
}
