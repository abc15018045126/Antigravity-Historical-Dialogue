use crate::models::ConversationSummary;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub last_backup_time: u64,
    pub last_backup_str: String,
    pub last_backup_type: String,
    pub total_sessions_backed_up: usize,
}

#[derive(Debug, Clone)]
pub struct BackupResult {
    pub success: bool,
    pub timestamp_str: String,
    pub message: String,
}

pub fn get_backup_directory() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(parent) = exe_path.parent()
    {
        let s = parent.to_string_lossy();
        if s.contains("target") && Path::new("Cargo.toml").exists() {
            return PathBuf::from("backups");
        }
        return parent.join("backups");
    }
    PathBuf::from("backups")
}

pub fn get_backup_info_path() -> PathBuf {
    get_backup_directory().join("backup_info.json")
}

pub fn load_backup_metadata() -> Option<BackupMetadata> {
    let path = get_backup_info_path();
    if !path.exists() {
        return None;
    }
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_backup_metadata(meta: &BackupMetadata) -> Result<(), String> {
    let dir = get_backup_directory();
    fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {e}"))?;
    let path = dir.join("backup_info.json");
    let json = serde_json::to_string_pretty(meta).map_err(|e| format!("序列化备份元数据失败: {e}"))?;
    fs::write(path, json).map_err(|e| format!("写入备份元数据文件失败: {e}"))
}

pub fn format_epoch_datetime(epoch: u64) -> String {
    if epoch == 0 {
        return "从未备份".to_string();
    }
    let sec_in_day = 86_400u64;
    let days_since_epoch = (epoch / sec_in_day) as i64;
    let day_seconds = epoch % sec_in_day;
    let hours = day_seconds / 3600;
    let minutes = (day_seconds % 3600) / 60;
    let seconds = day_seconds % 60;

    let z = days_since_epoch + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, hours, minutes, seconds)
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<usize> {
    if !src.exists() || !src.is_dir() {
        return Ok(0);
    }
    fs::create_dir_all(dst)?;
    let mut files_copied = 0;
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_child = entry.path();
            let dst_child = dst.join(entry.file_name());
            if src_child.is_dir() {
                files_copied += copy_dir_all(&src_child, &dst_child)?;
            } else if src_child.is_file() && fs::copy(&src_child, &dst_child).is_ok() {
                files_copied += 1;
            }
        }
    }
    Ok(files_copied)
}

pub fn perform_normal_backup(conversations: &[ConversationSummary]) -> Result<BackupResult, String> {
    let backup_dir = get_backup_directory();
    fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    let meta = load_backup_metadata();
    let last_time = meta.as_ref().map(|m| m.last_backup_time).unwrap_or(0);
    let last_time_str = meta.as_ref().map(|m| m.last_backup_str.clone()).unwrap_or_else(|| "从未备份".to_string());

    let now_epoch = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let now_str = format_epoch_datetime(now_epoch);

    // Only back up conversations modified after last_backup_time
    let to_backup: Vec<&ConversationSummary> = conversations
        .iter()
        .filter(|c| c.last_modified_epoch > last_time)
        .collect();

    if to_backup.is_empty() && last_time > 0 {
        return Ok(BackupResult {
            success: true,
            timestamp_str: now_str,
            message: format!("已是最新状态，无在上次备份（{last_time_str}）后修改的会话"),
        });
    }

    let mut copied_convs = 0;
    let mut total_files = 0;

    for conv in &to_backup {
        let target_conv_dir = backup_dir.join(&conv.source_name).join(&conv.id);
        if conv.path.is_file() {
            let parent = target_conv_dir.parent().unwrap_or(&backup_dir);
            if fs::create_dir_all(parent).is_ok() {
                let target_file = parent.join(conv.path.file_name().unwrap_or_default());
                if fs::copy(&conv.path, &target_file).is_ok() {
                    copied_convs += 1;
                    total_files += 1;
                }
            }
        } else if let Ok(count) = copy_dir_all(&conv.path, &target_conv_dir) {
            copied_convs += 1;
            total_files += count;
        }
    }

    let new_meta = BackupMetadata {
        last_backup_time: now_epoch,
        last_backup_str: now_str.clone(),
        last_backup_type: "普通备份".to_string(),
        total_sessions_backed_up: copied_convs,
    };
    save_backup_metadata(&new_meta)?;

    Ok(BackupResult {
        success: true,
        timestamp_str: now_str,
        message: format!("普通备份完成！已备份 {copied_convs} 个新修改会话（包含 {total_files} 个文件）"),
    })
}

pub fn perform_full_backup(conversations: &[ConversationSummary]) -> Result<BackupResult, String> {
    let backup_dir = get_backup_directory();
    fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    let now_epoch = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let now_str = format_epoch_datetime(now_epoch);

    let mut copied_convs = 0;
    let mut total_files = 0;

    for conv in conversations {
        let target_conv_dir = backup_dir.join(&conv.source_name).join(&conv.id);
        if conv.path.is_file() {
            let parent = target_conv_dir.parent().unwrap_or(&backup_dir);
            if fs::create_dir_all(parent).is_ok() {
                let target_file = parent.join(conv.path.file_name().unwrap_or_default());
                if fs::copy(&conv.path, &target_file).is_ok() {
                    copied_convs += 1;
                    total_files += 1;
                }
            }
        } else if let Ok(count) = copy_dir_all(&conv.path, &target_conv_dir) {
            copied_convs += 1;
            total_files += count;
        }
    }

    let new_meta = BackupMetadata {
        last_backup_time: now_epoch,
        last_backup_str: now_str.clone(),
        last_backup_type: "全部检查备份".to_string(),
        total_sessions_backed_up: copied_convs,
    };
    save_backup_metadata(&new_meta)?;

    Ok(BackupResult {
        success: true,
        timestamp_str: now_str,
        message: format!("全部检查备份完成！共检查并同步了 {copied_convs} 个会话（包含 {total_files} 个文件）"),
    })
}

pub fn open_backup_folder() {
    let dir = get_backup_directory();
    let _ = fs::create_dir_all(&dir);
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_epoch_datetime() {
        let dt = format_epoch_datetime(1_758_957_800);
        assert!(!dt.is_empty(), "Formatted datetime should not be empty");
        assert!(dt.contains('-') && dt.contains(':'), "Should have date and time separators");
    }

    #[test]
    fn test_backup_metadata_serialization() {
        let meta = BackupMetadata {
            last_backup_time: 1_234_567_890,
            last_backup_str: "2009-02-13 23:31:30".to_string(),
            last_backup_type: "普通备份".to_string(),
            total_sessions_backed_up: 5,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: BackupMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.last_backup_time, 1_234_567_890);
        assert_eq!(deserialized.last_backup_type, "普通备份");
    }
}
