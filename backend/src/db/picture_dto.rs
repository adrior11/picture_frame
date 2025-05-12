use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PictureDto {
    pub id: String,
    pub filename: String,
    pub added_at: i64,
}
