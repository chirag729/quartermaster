pub mod manager;
pub mod yaml_schema;
pub mod validate;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub is_builtin: bool,
    pub task_entries: Vec<BlueprintTaskEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintTaskEntry {
    pub task_id: String,
    pub enabled: bool,
    pub config_overrides: HashMap<String, Value>,
    pub order: u32,
}
