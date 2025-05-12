use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyCreateRequest {
    pub id: String,
    pub scope: String, // "ro" or "rw"
}
