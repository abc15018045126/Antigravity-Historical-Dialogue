use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub path: PathBuf,
    pub log_path: PathBuf,
    pub last_modified_epoch: u64,
    pub last_modified_str: String,
    pub step_count: usize,
    pub tool_count: usize,
    pub title_preview: String,
    pub source_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepRecord {
    pub id: usize,
    pub step_index: u64,
    pub source: String,
    pub step_type: String,
    pub status: String,
    pub created_at: String,
    pub content: Option<String>,
    pub thinking: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub exit_code: Option<i64>,
    pub raw_json: String,
}
