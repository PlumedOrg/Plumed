use super::steps::StepText;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub timestamp: i64,
    pub text: String,
    /// Plumed: the text after each processing step (see `steps`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepText>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryData {
    pub entries: Vec<HistoryEntry>,
    pub next_id: u64,
}

impl Default for HistoryData {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }
}
