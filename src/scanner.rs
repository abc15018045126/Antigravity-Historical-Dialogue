use crate::models::{ConversationSummary, StepRecord, ToolCall};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn get_search_directories() -> Vec<(&'static str, PathBuf)> {
    let mut dirs = Vec::new();
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\abc15"));

    // 1. Antigravity IDE (VS Code 插件版) - 优先展示 IDE
    let ide_dir = home.join(".gemini").join("antigravity-ide").join("brain");
    if ide_dir.exists() {
        dirs.push(("Antigravity IDE", ide_dir));
    }

    // 2. Antigravity 2.0 (独立桌面应用)
    let app_20_dir = home.join(".gemini").join("antigravity").join("brain");
    if app_20_dir.exists() {
        dirs.push(("Antigravity 2.0", app_20_dir));
    }

    // 3. Antigravity CLI (命令行版)
    let cli_dir = home.join(".gemini").join("antigravity-cli").join("brain");
    if cli_dir.exists() {
        dirs.push(("Antigravity CLI", cli_dir));
    }

    dirs
}

pub fn delete_conversation_folder(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.to_string_lossy().contains("session-")
        {
            return fs::remove_file(path).map_err(|e| format!("删除失败: {e}"));
        }
        return Err("安全拦截：禁止删除非会话文件".to_string());
    }
    if !path.is_dir() {
        return Err("目标不是有效文件夹".to_string());
    }
    if !path.join(".system_generated").exists()
        && !path.join("task.md").exists()
        && !path.join("walkthrough.md").exists()
    {
        return Err("安全拦截：该目录不包含系统会话特征，禁止删除".to_string());
    }
    fs::remove_dir_all(path).map_err(|e| format!("删除失败: {e}"))
}

pub fn scan_conversations() -> Vec<ConversationSummary> {
    let mut list = Vec::new();
    let mut seen_ids = HashSet::new();

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\abc15"));

    // 1. 扫描各个脑库目录 (Antigravity IDE / 2.0 / CLI)
    for (source_name, base_dir) in get_search_directories() {
        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let folder_name = match path.file_name() {
                    Some(name) => name.to_string_lossy().to_string(),
                    None => continue,
                };
                if folder_name.starts_with('.') || folder_name == "tempmediaStorage" {
                    continue;
                }
                if seen_ids.contains(&folder_name) {
                    continue;
                }

                let logs_dir = path.join(".system_generated").join("logs");
                let log_path = if logs_dir.join("transcript.jsonl").exists() {
                    logs_dir.join("transcript.jsonl")
                } else if logs_dir.join("overview.txt").exists() {
                    logs_dir.join("overview.txt")
                } else if path.join("task.md").exists() {
                    path.join("task.md")
                } else if path.join("walkthrough.md").exists() {
                    path.join("walkthrough.md")
                } else {
                    continue;
                };

                let metadata = match fs::metadata(&log_path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let last_modified_epoch = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let (step_count, tool_count, preview) = inspect_log_summary(&log_path);
                if step_count == 0 {
                    continue;
                }

                seen_ids.insert(folder_name.clone());
                let summary = ConversationSummary {
                    id: folder_name,
                    path: path.clone(),
                    log_path,
                    last_modified_epoch,
                    last_modified_str: format_epoch(last_modified_epoch),
                    step_count,
                    tool_count,
                    title_preview: preview,
                    source_name: source_name.to_string(),
                };
                list.push(summary);
            }
        }
    }

    // 2. 扫描 2025 年早期 Gemini CLI 对话记录 (session-*.json)
    let tmp_dir = home.join(".gemini").join("tmp");
    if tmp_dir.exists()
        && let Ok(entries) = fs::read_dir(&tmp_dir)
    {
        for entry in entries.flatten() {
            let chats_dir = entry.path().join("chats");
            if chats_dir.is_dir()
                && let Ok(chat_files) = fs::read_dir(&chats_dir)
            {
                for c_entry in chat_files.flatten() {
                    let json_path = c_entry.path();
                    if json_path.is_file()
                        && json_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.starts_with("session-") && n.ends_with(".json"))
                            .unwrap_or(false)
                    {
                        let metadata = match fs::metadata(&json_path) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };
                        let last_modified_epoch = metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        let (step_count, tool_count, preview) =
                            inspect_legacy_json_summary(&json_path);
                        if step_count == 0 {
                            continue;
                        }

                        let id = json_path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();

                        if seen_ids.contains(&id) {
                            continue;
                        }
                        seen_ids.insert(id.clone());

                        list.push(ConversationSummary {
                            id,
                            path: json_path.clone(),
                            log_path: json_path,
                            last_modified_epoch,
                            last_modified_str: format_epoch(last_modified_epoch),
                            step_count,
                            tool_count,
                            title_preview: preview,
                            source_name: "Gemini CLI 2025".to_string(),
                        });
                    }
                }
            }
        }
    }

    list.sort_by_key(|a| std::cmp::Reverse(a.last_modified_epoch));
    list
}

fn inspect_legacy_json_summary(path: &Path) -> (usize, usize, String) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return (0, 0, "无法打开日志".to_string()),
    };
    let reader = BufReader::new(file);
    if let Ok(v) = serde_json::from_reader::<_, serde_json::Value>(reader) {
        let messages = v.get("messages").and_then(|m| m.as_array());
        let step_count = messages.map(|m| m.len()).unwrap_or(0);
        let mut preview = String::new();
        if let Some(msgs) = messages {
            for m in msgs {
                if m.get("type").and_then(|s| s.as_str()) == Some("user")
                    && let Some(c) = m.get("content").and_then(|s| s.as_str())
                {
                    preview = clean_user_content(c);
                    break;
                }
            }
        }
        if preview.is_empty() {
            preview = "(无用户提问预览)".to_string();
        }
        (step_count, 0, preview)
    } else {
        (0, 0, "解析错误".to_string())
    }
}

fn inspect_log_summary(path: &Path) -> (usize, usize, String) {
    if path.extension().and_then(|e| e.to_str()) == Some("md") {
        let parent = path.parent().unwrap_or(path);
        let mut step_count = 0;
        if parent.join("task.md").exists() {
            step_count += 1;
        }
        if parent.join("implementation_plan.md").exists() {
            step_count += 1;
        }
        if parent.join("walkthrough.md").exists() {
            step_count += 1;
        }

        let preview = fs::read_to_string(path)
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| !l.is_empty() && !l.starts_with("<!--"))
                    .map(|l| {
                        let clean = l.trim_start_matches('#').trim();
                        clean_user_content(clean)
                    })
            })
            .unwrap_or_else(|| "历史任务快照 (Artifact)".to_string());

        return (step_count, 0, preview);
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return (0, 0, "无法打开日志".to_string()),
    };

    let reader = BufReader::new(file);
    let mut step_count = 0;
    let mut tool_count = 0;
    let mut first_preview = String::new();

    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        step_count += 1;

        let need_preview = first_preview.is_empty();
        let might_have_tools = trimmed.contains("\"tool_calls\"");

        if (need_preview || might_have_tools)
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed)
        {
            if let Some(tools) = v.get("tool_calls").and_then(|t| t.as_array()) {
                tool_count += tools.len();
            }

            if first_preview.is_empty()
                && let (Some("USER_INPUT"), Some(c)) = (
                    v.get("type").and_then(|s| s.as_str()),
                    v.get("content").and_then(|s| s.as_str()),
                )
            {
                let clean = clean_user_content(c);
                if !clean.is_empty() {
                    first_preview = clean;
                }
            }
        }
    }

    if first_preview.is_empty() {
        first_preview = "(无用户提问预览)".to_string();
    }

    (step_count, tool_count, first_preview)
}

pub fn clean_user_content(raw: &str) -> String {
    let mut content = raw;
    if let Some(start) = content.find("<USER_REQUEST>") {
        let after = &content[start + "<USER_REQUEST>".len()..];
        if let Some(end) = after.find("</USER_REQUEST>") {
            content = &after[..end];
        }
    }
    let trimmed = content.trim().replace('\r', "").replace('\n', " ");
    if trimmed.chars().count() > 60 {
        let s: String = trimmed.chars().take(60).collect();
        format!("{}...", s)
    } else {
        trimmed
    }
}

pub fn load_steps_from_file(log_path: &Path) -> Vec<StepRecord> {
    if log_path.extension().and_then(|e| e.to_str()) == Some("md") {
        let parent = log_path.parent().unwrap_or(log_path);
        let mut steps = Vec::new();
        let mut next_id = 0;

        let task_file = parent.join("task.md");
        if let Ok(content) = fs::read_to_string(&task_file) {
            steps.push(StepRecord {
                id: next_id,
                step_index: next_id as u64,
                source: "USER_EXPLICIT".to_string(),
                step_type: "USER_INPUT".to_string(),
                status: "DONE".to_string(),
                created_at: "历史任务需求 (Task)".to_string(),
                content: Some(content.clone()),
                thinking: None,
                tool_calls: Vec::new(),
                exit_code: None,
                raw_json: serde_json::json!({ "file": "task.md", "content": content }).to_string(),
            });
            next_id += 1;
        }

        let plan_file = parent.join("implementation_plan.md");
        if let Ok(content) = fs::read_to_string(&plan_file) {
            steps.push(StepRecord {
                id: next_id,
                step_index: next_id as u64,
                source: "MODEL".to_string(),
                step_type: "PLANNER_RESPONSE".to_string(),
                status: "DONE".to_string(),
                created_at: "实施方案 (Plan)".to_string(),
                content: Some(content.clone()),
                thinking: None,
                tool_calls: Vec::new(),
                exit_code: None,
                raw_json: serde_json::json!({ "file": "implementation_plan.md", "content": content }).to_string(),
            });
            next_id += 1;
        }

        let walk_file = parent.join("walkthrough.md");
        if let Ok(content) = fs::read_to_string(&walk_file) {
            steps.push(StepRecord {
                id: next_id,
                step_index: next_id as u64,
                source: "MODEL".to_string(),
                step_type: "PLANNER_RESPONSE".to_string(),
                status: "DONE".to_string(),
                created_at: "执行成果与演示 (Walkthrough)".to_string(),
                content: Some(content.clone()),
                thinking: None,
                tool_calls: Vec::new(),
                exit_code: None,
                raw_json: serde_json::json!({ "file": "walkthrough.md", "content": content }).to_string(),
            });
        }
        return steps;
    }

    if log_path.extension().and_then(|e| e.to_str()) == Some("json") {
        let file = match File::open(log_path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let reader = BufReader::new(file);
        if let Ok(v) = serde_json::from_reader::<_, serde_json::Value>(reader) {
            let mut steps = Vec::new();
            if let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) {
                for (idx, msg) in msgs.iter().enumerate() {
                    let is_user = msg.get("type").and_then(|s| s.as_str()) == Some("user");
                    let content = msg.get("content").and_then(|s| s.as_str()).map(|s| s.to_string());
                    let timestamp = msg.get("timestamp").and_then(|s| s.as_str()).unwrap_or("-").to_string();
                    steps.push(StepRecord {
                        id: idx,
                        step_index: idx as u64,
                        source: if is_user { "USER_EXPLICIT".to_string() } else { "MODEL".to_string() },
                        step_type: if is_user { "USER_INPUT".to_string() } else { "PLANNER_RESPONSE".to_string() },
                        status: "DONE".to_string(),
                        created_at: timestamp,
                        content,
                        thinking: None,
                        tool_calls: Vec::new(),
                        exit_code: None,
                        raw_json: msg.to_string(),
                    });
                }
            }
            return steps;
        }
    }

    let file = match File::open(log_path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut steps = Vec::new();

    for (fallback_idx, line) in reader.lines().map_while(Result::ok).enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let step_index = parsed
            .get("step_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(fallback_idx as u64);

        let source = parsed
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();

        let step_type = parsed
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();

        let status = parsed
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("DONE")
            .to_string();

        let created_at = parsed
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let content = parsed
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let thinking = parsed
            .get("thinking")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let exit_code = parsed.get("exit_code").and_then(|v| v.as_i64());

        let mut tool_calls = Vec::new();
        if let Some(arr) = parsed.get("tool_calls").and_then(|v| v.as_array()) {
            for item in arr {
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown_tool")
                    .to_string();
                let args = item.get("args").cloned().unwrap_or(serde_json::Value::Null);
                tool_calls.push(ToolCall { name, args });
            }
        }

        let raw_json = trimmed.to_string();

        steps.push(StepRecord {
            id: fallback_idx,
            step_index,
            source,
            step_type,
            status,
            created_at,
            content,
            thinking,
            tool_calls,
            exit_code,
            raw_json,
        });
    }

    steps
}

fn format_epoch(epoch: u64) -> String {
    if epoch == 0 {
        return "-".to_string();
    }
    let diff = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().saturating_sub(epoch))
        .unwrap_or(0);

    if diff < 60 {
        format!("{}秒前", diff)
    } else if diff < 3600 {
        format!("{}分钟前", diff / 60)
    } else if diff < 86400 {
        format!("{}小时前", diff / 3600)
    } else {
        format!("{}天前", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_large_file_loading_and_unique_ids() {
        let path = PathBuf::from(
            r"C:\Users\abc15\.gemini\antigravity-ide\brain\1f958eab-ec01-450b-8bcc-3f82f30aa2cb\.system_generated\logs\transcript.jsonl",
        );
        if !path.exists() {
            return;
        }

        let steps = load_steps_from_file(&path);
        assert!(!steps.is_empty(), "Steps should not be empty");

        // Verify that every step id is strictly unique
        let mut ids = std::collections::HashSet::new();
        for step in &steps {
            assert!(ids.insert(step.id), "Step id must be unique: {}", step.id);
        }
    }

    #[test]
    fn test_scan_conversations_uniqueness() {
        let convs = scan_conversations();
        assert!(!convs.is_empty(), "Should discover conversations");
        let mut seen = std::collections::HashSet::new();
        for c in &convs {
            assert!(seen.insert(c.id.clone()), "Duplicate conversation ID in scan: {}", c.id);
        }
    }
}
