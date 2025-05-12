use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyInfo {
    pub id: String,
    pub scope: String,
    pub created: i64, // epoch seconds
}
