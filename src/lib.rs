pub mod backup;
pub mod i18n;
pub mod icon_data;
pub mod models;
pub mod scanner;

pub use backup::*;
pub use i18n::Language;
pub use icon_data::*;
pub use models::*;
pub use scanner::*;

use dioxus::prelude::*;

pub fn load_app_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    let img = image::load_from_memory(include_bytes!("../icon.png")).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    dioxus::desktop::tao::window::Icon::from_rgba(img.into_raw(), width, height).ok()
}

pub fn launch_desktop() {
    let app_icon = load_app_icon();
    let mut cfg = dioxus::desktop::Config::new();
    if let Some(ref icon) = app_icon {
        cfg = cfg.with_icon(icon.clone());
    }
    cfg = cfg.with_window(
        dioxus::desktop::WindowBuilder::new()
            .with_title("Antigravity Historical Dialogue")
            .with_window_icon(app_icon),
    );
    dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(|| {
        rsx! {
            HistoricalDialogueViewer {}
        }
    });
}

#[derive(Props, Clone, PartialEq, Default)]
pub struct HistoricalDialogueProps {
    #[props(default)]
    pub default_lang: Option<Language>,
    #[props(default = String::new())]
    pub class: String,
    #[props(default)]
    pub on_close: Option<EventHandler<()>>,
}

#[derive(Props, Clone, PartialEq)]
pub struct HistoricalDialogueTabProps {
    #[props(default = "历史会话".to_string())]
    pub label: String,
    #[props(default = "📜".to_string())]
    pub icon: String,
    #[props(default = false)]
    pub default_open: bool,
}

#[component]
pub fn HistoricalDialogueTab(props: HistoricalDialogueTabProps) -> Element {
    let mut is_open = use_signal(|| props.default_open);
    rsx! {
        button {
            class: "agy-tab-trigger-btn",
            onclick: move |_| is_open.toggle(),
            span { class: "agy-tab-icon", "{props.icon}" }
            span { "{props.label}" }
        }
        if *is_open.read() {
            div { class: "agy-embedded-modal-overlay",
                div { class: "agy-embedded-modal-body",
                    div { class: "agy-embedded-modal-header",
                        span { class: "agy-embedded-modal-title", "{props.icon} {props.label}" }
                        button {
                            class: "agy-embedded-modal-close-btn",
                            onclick: move |_| is_open.set(false),
                            "✕"
                        }
                    }
                    div { class: "agy-embedded-modal-content",
                        HistoricalDialogueViewer {
                            on_close: move |_| is_open.set(false),
                        }
                    }
                }
            }
        }
    }
}

const RESIZER_SCRIPT: &str = r#"
(function() {
    if (window.__agy_resizer_initialized) return;
    window.__agy_resizer_initialized = true;

    try {
        const saved = localStorage.getItem('agy_sidebar_width');
        if (saved) {
            const parsed = parseInt(saved, 10);
            if (parsed >= 200 && parsed <= window.innerWidth - 300) {
                document.documentElement.style.setProperty('--sidebar-width', parsed + 'px');
            }
        }
    } catch (_) {}

    let isDragging = false;
    let startX = 0;
    let startWidth = 0;

    document.addEventListener('mousedown', function(e) {
        const resizer = e.target.closest('#sidebar-resizer');
        if (!resizer) return;
        const sidebar = document.querySelector('.sidebar');
        if (!sidebar) return;

        e.preventDefault();
        isDragging = true;
        startX = e.clientX;
        startWidth = sidebar.getBoundingClientRect().width;
        document.body.classList.add('resizing');
        resizer.classList.add('active');
    });

    window.addEventListener('mousemove', function(e) {
        if (!isDragging) return;
        e.preventDefault();
        const deltaX = e.clientX - startX;
        let newWidth = startWidth + deltaX;

        const minWidth = 220;
        const maxWidth = Math.max(minWidth + 100, window.innerWidth - 320);
        if (newWidth < minWidth) newWidth = minWidth;
        if (newWidth > maxWidth) newWidth = maxWidth;

        document.documentElement.style.setProperty('--sidebar-width', newWidth + 'px');
        const sidebar = document.querySelector('.sidebar');
        if (sidebar) {
            sidebar.style.width = newWidth + 'px';
        }
    });

    window.addEventListener('mouseup', function() {
        if (isDragging) {
            isDragging = false;
            document.body.classList.remove('resizing');
            const resizer = document.getElementById('sidebar-resizer');
            if (resizer) resizer.classList.remove('active');
            const sidebar = document.querySelector('.sidebar');
            if (sidebar) {
                const currentWidth = Math.round(sidebar.getBoundingClientRect().width);
                try {
                    localStorage.setItem('agy_sidebar_width', currentWidth);
                } catch (_) {}
            }
        }
    });

    document.addEventListener('dblclick', function(e) {
        const resizer = e.target.closest('#sidebar-resizer');
        if (!resizer) return;
        e.preventDefault();
        const defaultWidth = 380;
        document.documentElement.style.setProperty('--sidebar-width', defaultWidth + 'px');
        const sidebar = document.querySelector('.sidebar');
        if (sidebar) {
            sidebar.style.width = defaultWidth + 'px';
        }
        try {
            localStorage.setItem('agy_sidebar_width', defaultWidth);
        } catch (_) {}
    });
})();
"#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FilterMode {
    All,
    ToolsOnly,
    ChatOnly,
    ThinkingOnly,
}

fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

fn format_pretty_json(raw: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string())
    } else {
        raw.to_string()
    }
}

#[component]
pub fn HistoricalDialogueViewer(props: HistoricalDialogueProps) -> Element {
    let mut current_lang = use_signal(|| props.default_lang.unwrap_or_else(Language::default_lang));
    let lang = *current_lang.read();
    let is_en = lang.is_en();

    let mut conversations = use_signal(scan_conversations);
    let mut selected_id = use_signal(|| -> Option<String> {
        conversations.read().first().map(|c| c.id.clone())
    });
    let mut search_query = use_signal(String::new);
    let mut filter_mode = use_signal(|| FilterMode::All);
    let mut current_page = use_signal(|| 1usize);
    let mut page_size = use_signal(|| 50usize);
    let mut page_input_text = use_signal(|| "1".to_string());
    let mut sidebar_page = use_signal(|| 1usize);
    let mut sidebar_page_input = use_signal(|| "1".to_string());
    let sidebar_page_size = 20usize;
    let mut sidebar_single_page = use_signal(|| false);
    let mut main_single_page = use_signal(|| false);
    let mut delete_modal_conv = use_signal(|| -> Option<crate::models::ConversationSummary> { None });
    let mut delete_error = use_signal(|| -> Option<String> { None });
    let mut expanded_raw = use_signal(std::collections::HashSet::<usize>::new);
    let mut expanded_thinking = use_signal(std::collections::HashSet::<usize>::new);
    let mut backup_metadata = use_signal(load_backup_metadata);
    let mut is_backing_up = use_signal(|| false);
    let mut backup_notice = use_signal(|| -> Option<BackupResult> { None });
    let mut show_search = use_signal(|| false);
    let mut show_backup_card = use_signal(|| false);
    let mut is_sidebar_collapsed = use_signal(|| false);
    let mut show_conv_details = use_signal(|| false);
    let mut file_search_query = use_signal(String::new);
    let mut show_file_search = use_signal(|| false);

    use_effect(move || {
        let _ = dioxus::prelude::document::eval(RESIZER_SCRIPT);
    });

    let current_conv = use_memo(move || {
        let convs = conversations.read();
        let sel = selected_id.read();
        sel.as_ref().and_then(|id| convs.iter().find(|c| &c.id == id).cloned())
    });

    let steps = use_memo(move || {
        if let Some(conv) = current_conv.read().as_ref() {
            load_steps_from_file(&conv.log_path)
        } else {
            Vec::new()
        }
    });

    let filtered_steps = use_memo(move || {
        let all_steps = steps.read();
        let mode = *filter_mode.read();
        let fq = file_search_query.read().trim().to_lowercase();
        all_steps
            .iter()
            .filter(|s| {
                let mode_match = match mode {
                    FilterMode::All => true,
                    FilterMode::ToolsOnly => {
                        !s.tool_calls.is_empty()
                            || s.step_type == "RUN_COMMAND"
                            || s.step_type == "VIEW_FILE"
                            || s.step_type == "REPLACE_FILE_CONTENT"
                            || s.step_type == "WRITE_TO_FILE"
                    }
                    FilterMode::ChatOnly => {
                        s.step_type == "USER_INPUT"
                            || (s.step_type == "PLANNER_RESPONSE" && s.content.is_some())
                    }
                    FilterMode::ThinkingOnly => s.thinking.is_some(),
                };
                if !mode_match {
                    return false;
                }
                if fq.is_empty() {
                    true
                } else {
                    s.content.as_ref().map(|c| c.to_lowercase().contains(&fq)).unwrap_or(false)
                        || s.thinking.as_ref().map(|t| t.to_lowercase().contains(&fq)).unwrap_or(false)
                        || s.raw_json.to_lowercase().contains(&fq)
                }
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    let total_tools_in_active: usize = steps.read().iter().map(|s| s.tool_calls.len()).sum();

    // Right pane steps pagination calculations
    let is_main_single = *main_single_page.read();
    let total_filtered = filtered_steps.read().len();
    let page_size_num = *page_size.read();
    let total_pages = if total_filtered == 0 || is_main_single {
        1
    } else {
        total_filtered.div_ceil(page_size_num)
    };
    let cur_page = (*current_page.read()).clamp(1, total_pages);
    let start_idx = if total_filtered == 0 || is_main_single { 0 } else { (cur_page - 1) * page_size_num };
    let end_idx = if is_main_single { total_filtered } else { (start_idx + page_size_num).min(total_filtered) };

    let page_steps = if total_filtered == 0 {
        Vec::new()
    } else {
        filtered_steps.read()[start_idx..end_idx].to_vec()
    };

    // Sidebar conversation filtering & pagination
    let is_sidebar_single = *sidebar_single_page.read();
    let filtered_convs = use_memo(move || {
        let q = search_query.read().to_lowercase();
        conversations
            .read()
            .iter()
            .filter(|c| {
                if q.is_empty() {
                    true
                } else {
                    c.id.to_lowercase().contains(&q)
                        || c.title_preview.to_lowercase().contains(&q)
                }
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    let total_convs = filtered_convs.read().len();
    let total_conv_pages = if total_convs == 0 || is_sidebar_single {
        1
    } else {
        total_convs.div_ceil(sidebar_page_size)
    };
    let cur_conv_page = (*sidebar_page.read()).clamp(1, total_conv_pages);
    let conv_start_idx = if total_convs == 0 || is_sidebar_single {
        0
    } else {
        (cur_conv_page - 1) * sidebar_page_size
    };
    let conv_end_idx = if is_sidebar_single {
        total_convs
    } else {
        (conv_start_idx + sidebar_page_size).min(total_convs)
    };
    let conv_display_start = if total_convs == 0 { 0 } else { conv_start_idx + 1 };

    let page_convs = if total_convs == 0 {
        Vec::new()
    } else {
        filtered_convs.read()[conv_start_idx..conv_end_idx].to_vec()
    };

    rsx! {
        style { {include_str!("style.css")} }
        div { class: "app-container {props.class}",
            // Sidebar
            div { class: if *is_sidebar_collapsed.read() { "sidebar collapsed" } else { "sidebar" },
                div { class: "sidebar-header-compact",
                    div { class: "header-toolbar",
                        img {
                            class: "app-brand-icon",
                            src: "{icon_data::APP_ICON_DATA_URL}",
                            alt: "Antigravity",
                            title: "Antigravity Historical Dialogue",
                        }
                        button {
                            class: "toolbar-btn normal-backup-btn",
                            title: {
                                let time_hint = backup_metadata
                                    .read()
                                    .as_ref()
                                    .map(|m| m.last_backup_str.clone())
                                    .unwrap_or_else(|| "从未备份".to_string());
                                format!("普通备份：仅备份上次备份时间（{time_hint}）之后修改的会话")
                            },
                            disabled: *is_backing_up.read(),
                            onclick: move |_| {
                                is_backing_up.set(true);
                                let convs = conversations.read().clone();
                                let res = perform_normal_backup(&convs);
                                is_backing_up.set(false);
                                match res {
                                    Ok(r) => {
                                        backup_metadata.set(load_backup_metadata());
                                        backup_notice.set(Some(r));
                                        show_backup_card.set(true);
                                    }
                                    Err(e) => {
                                        backup_notice.set(Some(BackupResult {
                                            success: false,
                                            timestamp_str: String::new(),
                                            message: e,
                                        }));
                                        show_backup_card.set(true);
                                    }
                                }
                            },
                            if *is_backing_up.read() {
                                "⏳ 备份中..."
                            } else {
                                "📦 普通备份"
                            }
                        }
                        button {
                            class: "toolbar-btn full-backup-btn",
                            title: "全部检查备份：完整扫描并同步所有会话至备份目录",
                            disabled: *is_backing_up.read(),
                            onclick: move |_| {
                                is_backing_up.set(true);
                                let convs = conversations.read().clone();
                                let res = perform_full_backup(&convs);
                                is_backing_up.set(false);
                                match res {
                                    Ok(r) => {
                                        backup_metadata.set(load_backup_metadata());
                                        backup_notice.set(Some(r));
                                        show_backup_card.set(true);
                                    }
                                    Err(e) => {
                                        backup_notice.set(Some(BackupResult {
                                            success: false,
                                            timestamp_str: String::new(),
                                            message: e,
                                        }));
                                        show_backup_card.set(true);
                                    }
                                }
                            },
                            if *is_backing_up.read() {
                                "⏳ 备份中..."
                            } else {
                                "🛡️ 全部检查"
                            }
                        }
                        button {
                            class: "toolbar-btn refresh-btn",
                            title: "重新扫描磁盘三大数据源会话",
                            onclick: move |_| {
                                conversations.set(scan_conversations());
                                current_page.set(1);
                                page_input_text.set("1".to_string());
                                sidebar_page.set(1);
                                sidebar_page_input.set("1".to_string());
                            },
                            "🔄 刷新"
                        }
                        button {
                            class: if *show_search.read() || !search_query.read().is_empty() {
                                "toolbar-btn search-toggle-btn active"
                            } else {
                                "toolbar-btn search-toggle-btn"
                            },
                            title: "展开/收起搜索框",
                            onclick: move |_| {
                                show_search.toggle();
                            },
                            if !search_query.read().is_empty() {
                                "🔍 搜索(已筛选)"
                            } else {
                                "🔍 搜索"
                            }
                        }
                        button {
                            class: if *show_backup_card.read() {
                                "toolbar-btn folder-btn active"
                            } else {
                                "toolbar-btn folder-btn"
                            },
                            title: "查看备份详情与打开备份目录",
                            onclick: move |_| {
                                let currently_open = *show_backup_card.read();
                                if currently_open {
                                    show_backup_card.set(false);
                                } else {
                                    if backup_notice.read().is_none() {
                                        if let Some(m) = backup_metadata.read().as_ref() {
                                            backup_notice.set(Some(BackupResult {
                                                success: true,
                                                timestamp_str: m.last_backup_str.clone(),
                                                message: format!("备份目录已就绪，最后一次备份为【{}】（已备份 {} 个会话）", m.last_backup_type, m.total_sessions_backed_up),
                                            }));
                                        } else {
                                            backup_notice.set(Some(BackupResult {
                                                success: true,
                                                timestamp_str: String::new(),
                                                message: "尚未进行过备份。请点击【普通备份】或【全部检查】开始备份。".to_string(),
                                            }));
                                        }
                                    }
                                    show_backup_card.set(true);
                                }
                            },
                            "📂"
                        }
                        button {
                            class: "toolbar-btn lang-toggle-btn",
                            title: if is_en { "Switch to Chinese (切换为中文)" } else { "切换为英文 (Switch to English)" },
                            onclick: move |_| {
                                let next_lang = current_lang.cloned().toggle();
                                current_lang.set(next_lang);
                            },
                            "{lang.button_label()}"
                        }
                        if let Some(on_close) = props.on_close {
                            button {
                                class: "toolbar-btn close-embed-btn",
                                title: if is_en { "Close viewer" } else { "关闭视图" },
                                onclick: move |_| on_close.call(()),
                                "✕"
                            }
                        }
                        button {
                            class: "toolbar-btn collapse-sidebar-btn",
                            title: "收起左侧栏",
                            onclick: move |_| {
                                is_sidebar_collapsed.set(true);
                            },
                            "◀"
                        }
                    }

                    if *show_search.read() {
                        div { class: "collapsible-search-box",
                            input {
                                placeholder: "搜索会话 ID 或内容...",
                                value: "{search_query}",
                                oninput: move |e| {
                                    search_query.set(e.value());
                                    current_page.set(1);
                                    page_input_text.set("1".to_string());
                                    sidebar_page.set(1);
                                    sidebar_page_input.set("1".to_string());
                                },
                            }
                            if !search_query.read().is_empty() {
                                button {
                                    class: "clear-search-btn",
                                    title: "清空搜索",
                                    onclick: move |_| {
                                        search_query.set(String::new());
                                        current_page.set(1);
                                        page_input_text.set("1".to_string());
                                        sidebar_page.set(1);
                                        sidebar_page_input.set("1".to_string());
                                    },
                                    "✕"
                                }
                            }
                        }
                    }

                    if *show_backup_card.read() && let Some(notice) = backup_notice.read().as_ref() {
                        div { class: "backup-notification-card",
                            div { class: "backup-notification-main",
                                span {
                                    class: if notice.success { "notice-icon success" } else { "notice-icon error" },
                                    if notice.success { "✅" } else { "⚠️" }
                                }
                                div { class: "notice-text-wrap",
                                    div { class: "notice-msg", "{notice.message}" }
                                    if !notice.timestamp_str.is_empty() {
                                        div { class: "notice-meta", "备份时间: {notice.timestamp_str}" }
                                    }
                                }
                            }
                            div { class: "notice-actions",
                                button {
                                    class: "notice-open-dir-btn",
                                    onclick: move |_| {
                                        open_backup_folder();
                                    },
                                    "📂 打开备份文件夹"
                                }
                                button {
                                    class: "notice-close-btn",
                                    onclick: move |_| {
                                        show_backup_card.set(false);
                                    },
                                    "✕ 关闭"
                                }
                            }
                        }
                    }
                }

                // Sidebar Pagination Bar
                div { class: "sidebar-pagination-bar",
                    div { class: "sidebar-page-info",
                        if is_sidebar_single {
                            span { class: "single-page-badge", "单页全部 (共 {total_convs})" }
                        } else {
                            span { "第 {conv_display_start}-{conv_end_idx} 项 (共 {total_convs})" }
                        }
                    }
                    div { class: "sidebar-page-nav",
                        button {
                            class: if is_sidebar_single { "sidebar-page-btn single-toggle active" } else { "sidebar-page-btn single-toggle" },
                            title: if is_sidebar_single { "切换回分页浏览（每页 20 条）" } else { "单页显示全部会话" },
                            onclick: move |_| {
                                let cur = *sidebar_single_page.read();
                                sidebar_single_page.set(!cur);
                                sidebar_page.set(1);
                                sidebar_page_input.set("1".to_string());
                            },
                            if is_sidebar_single {
                                "📑 分页显示"
                            } else {
                                "📄 单页显示"
                            }
                        }
                        button {
                            class: "sidebar-page-btn",
                            disabled: is_sidebar_single || cur_conv_page <= 1,
                            title: "首页",
                            onclick: move |_| {
                                sidebar_page.set(1);
                                sidebar_page_input.set("1".to_string());
                            },
                            "⏮"
                        }
                        button {
                            class: "sidebar-page-btn",
                            disabled: is_sidebar_single || cur_conv_page <= 1,
                            title: "上一页",
                            onclick: move |_| {
                                let p = cur_conv_page.saturating_sub(1).max(1);
                                sidebar_page.set(p);
                                sidebar_page_input.set(p.to_string());
                            },
                            "◀"
                        }
                        div { class: "sidebar-jump-wrap",
                            span { class: "sidebar-jump-label", "第" }
                            input {
                                class: "sidebar-jump-input",
                                disabled: is_sidebar_single,
                                r#type: "number",
                                min: "1",
                                max: "{total_conv_pages}",
                                value: "{sidebar_page_input}",
                                oninput: move |e| {
                                    sidebar_page_input.set(e.value());
                                },
                                onkeydown: move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter
                                        && let Ok(p) = sidebar_page_input.cloned().parse::<usize>()
                                    {
                                        let clamped = p.clamp(1, total_conv_pages);
                                        sidebar_page.set(clamped);
                                        sidebar_page_input.set(clamped.to_string());
                                    }
                                },
                            }
                            span { class: "sidebar-jump-total", "/ {total_conv_pages} 页" }
                            button {
                                class: "sidebar-jump-btn",
                                disabled: is_sidebar_single,
                                title: "跳转到指定页",
                                onclick: move |_| {
                                    if let Ok(p) = sidebar_page_input.cloned().parse::<usize>() {
                                        let clamped = p.clamp(1, total_conv_pages);
                                        sidebar_page.set(clamped);
                                        sidebar_page_input.set(clamped.to_string());
                                    }
                                },
                                "跳转"
                            }
                        }
                        button {
                            class: "sidebar-page-btn",
                            disabled: is_sidebar_single || cur_conv_page >= total_conv_pages,
                            title: "下一页",
                            onclick: move |_| {
                                let p = (cur_conv_page + 1).min(total_conv_pages);
                                sidebar_page.set(p);
                                sidebar_page_input.set(p.to_string());
                            },
                            "▶"
                        }
                        button {
                            class: "sidebar-page-btn",
                            disabled: is_sidebar_single || cur_conv_page >= total_conv_pages,
                            title: "末页",
                            onclick: move |_| {
                                sidebar_page.set(total_conv_pages);
                                sidebar_page_input.set(total_conv_pages.to_string());
                            },
                            "⏭"
                        }
                    }
                }

                div { class: "conversation-list",
                    if page_convs.is_empty() {
                        div { class: "empty-conv-hint", "未找到匹配的会话记录" }
                    }
                    for conv in page_convs {
                        {
                            let is_active = selected_id.read().as_ref() == Some(&conv.id);
                            let conv_id = conv.id.clone();
                            let conv_for_del = conv.clone();
                            rsx! {
                                div {
                                    class: if is_active { "conv-card active" } else { "conv-card" },
                                    onclick: move |_| {
                                        selected_id.set(Some(conv_id.clone()));
                                        expanded_raw.write().clear();
                                        expanded_thinking.write().clear();
                                        file_search_query.set(String::new());
                                        current_page.set(1);
                                        page_input_text.set("1".to_string());
                                    },
                                    div { class: "conv-card-top",
                                        span {
                                            class: if conv.source_name.contains("2.0") {
                                                "source-tag source-20"
                                            } else if conv.source_name.contains("IDE") {
                                                "source-tag source-ide"
                                            } else if conv.source_name.contains("2025") {
                                                "source-tag source-legacy"
                                            } else {
                                                "source-tag source-cli"
                                            },
                                            "{conv.source_name}"
                                        }
                                        div { class: "conv-card-top-right",
                                            span { class: "time-tag", "{conv.last_modified_str}" }
                                            button {
                                                class: "card-delete-icon-btn",
                                                title: "删除该会话",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    delete_error.set(None);
                                                    delete_modal_conv.set(Some(conv_for_del.clone()));
                                                },
                                                "🗑️"
                                            }
                                        }
                                    }
                                    div { class: "conv-preview", "{conv.title_preview}" }
                                    div { class: "conv-card-meta",
                                        span { class: "stat-badge", "步数: {conv.step_count}" }
                                        span { class: "stat-badge tool-badge", "工具: {conv.tool_count}" }
                                    }
                                    div { class: "conv-id-sub", "{conv.id}" }
                                }
                            }
                        }
                    }
                }
            }

            // Draggable Divider / Resizer
            div {
                id: "sidebar-resizer",
                class: if *is_sidebar_collapsed.read() { "sidebar-resizer collapsed" } else { "sidebar-resizer" },
                title: "拖动调整侧边栏宽度 (双击重置)",
            }

            // Main Content Area
            div { class: "main-pane",
                if let Some(conv) = current_conv.read().as_ref() {
                    div { class: "main-header",
                        if *is_sidebar_collapsed.read() {
                            button {
                                class: "expand-sidebar-btn",
                                title: "展开左侧栏",
                                onclick: move |_| {
                                    is_sidebar_collapsed.set(false);
                                },
                                "▶ 展开侧栏"
                            }
                        }
                        div { class: "header-left",
                            div { class: "conv-title-row",
                                h3 { "会话日志: " span { class: "uuid-text", "{conv.id}" } }
                                button {
                                    class: if *show_conv_details.read() { "details-toggle-btn active" } else { "details-toggle-btn" },
                                    title: if *show_conv_details.read() { "收起详细信息" } else { "点击展开来源、步数与文件路径等详情" },
                                    onclick: move |_| {
                                        show_conv_details.toggle();
                                    },
                                    if *show_conv_details.read() {
                                        "ℹ️ 详情 ▴"
                                    } else {
                                        "ℹ️ 详情 ▾"
                                    }
                                }
                                button {
                                    class: "delete-conv-btn",
                                    title: "从磁盘永久删除此会话记录",
                                    onclick: {
                                        let conv_to_del = conv.clone();
                                        move |_| {
                                            delete_error.set(None);
                                            delete_modal_conv.set(Some(conv_to_del.clone()));
                                        }
                                    },
                                    "🗑️ 删除此会话"
                                }
                            }
                            if *show_conv_details.read() {
                                div { class: "conv-sub-details",
                                    span { class: "meta-chip", "来源: {conv.source_name}" }
                                    span { class: "meta-chip", "总步数: {steps.read().len()}" }
                                    span { class: "meta-chip accent", "工具调用次数: {total_tools_in_active}" }
                                    span { class: "meta-chip path-chip", title: "{conv.log_path.display()}", "文件: {conv.log_path.file_name().unwrap_or_default().to_string_lossy()}" }
                                }
                            }
                        }
                        div { class: "filter-tabs",
                            button {
                                class: if *filter_mode.read() == FilterMode::All { "tab active" } else { "tab" },
                                onclick: move |_| {
                                    filter_mode.set(FilterMode::All);
                                    current_page.set(1);
                                    page_input_text.set("1".to_string());
                                },
                                "全部 ({steps.read().len()})"
                            }
                            button {
                                class: if *filter_mode.read() == FilterMode::ToolsOnly { "tab active" } else { "tab" },
                                onclick: move |_| {
                                    filter_mode.set(FilterMode::ToolsOnly);
                                    current_page.set(1);
                                    page_input_text.set("1".to_string());
                                },
                                "🛠️ 仅工具调用"
                            }
                            button {
                                class: if *filter_mode.read() == FilterMode::ChatOnly { "tab active" } else { "tab" },
                                onclick: move |_| {
                                    filter_mode.set(FilterMode::ChatOnly);
                                    current_page.set(1);
                                    page_input_text.set("1".to_string());
                                },
                                "💬 仅对话"
                            }
                            button {
                                class: if *filter_mode.read() == FilterMode::ThinkingOnly { "tab active" } else { "tab" },
                                onclick: move |_| {
                                    filter_mode.set(FilterMode::ThinkingOnly);
                                    current_page.set(1);
                                    page_input_text.set("1".to_string());
                                },
                                "🧠 仅思考链"
                            }
                        }
                    }

                    // Pinned Top Pagination Bar with Direct Page Jump Input
                    div { class: "top-pagination-bar",
                        div { class: "pagination-info",
                            span { "显示第 {start_idx + 1} - {end_idx} 步 (共 {total_filtered} 步)" }
                        }
                        div { class: "pagination-nav",
                            button {
                                class: "page-btn",
                                disabled: cur_page <= 1,
                                onclick: move |_| {
                                    current_page.set(1);
                                    page_input_text.set("1".to_string());
                                },
                                "⏮ 首页"
                            }
                            button {
                                class: "page-btn",
                                disabled: cur_page <= 1,
                                onclick: move |_| {
                                    let p = cur_page.saturating_sub(1).max(1);
                                    current_page.set(p);
                                    page_input_text.set(p.to_string());
                                },
                                "◀ 上一页"
                            }
                            div { class: "page-jump-container",
                                span { class: "jump-label", "第" }
                                input {
                                    class: "page-jump-input",
                                    r#type: "number",
                                    min: "1",
                                    max: "{total_pages}",
                                    value: "{page_input_text}",
                                    oninput: move |e| {
                                        page_input_text.set(e.value());
                                    },
                                    onkeydown: move |e: KeyboardEvent| {
                                        if e.key() == Key::Enter
                                            && let Ok(p) = page_input_text.cloned().parse::<usize>()
                                        {
                                            let clamped = p.clamp(1, total_pages);
                                            current_page.set(clamped);
                                            page_input_text.set(clamped.to_string());
                                        }
                                    },
                                }
                                span { class: "jump-total", "/ {total_pages} 页" }
                                button {
                                    class: "page-jump-btn",
                                    onclick: move |_| {
                                        if let Ok(p) = page_input_text.cloned().parse::<usize>() {
                                            let clamped = p.clamp(1, total_pages);
                                            current_page.set(clamped);
                                            page_input_text.set(clamped.to_string());
                                        }
                                    },
                                    "跳转"
                                }
                            }
                            button {
                                class: "page-btn",
                                disabled: cur_page >= total_pages,
                                onclick: move |_| {
                                    let p = (cur_page + 1).min(total_pages);
                                    current_page.set(p);
                                    page_input_text.set(p.to_string());
                                },
                                "下一页 ▶"
                            }
                            button {
                                class: "page-btn",
                                disabled: cur_page >= total_pages,
                                onclick: move |_| {
                                    current_page.set(total_pages);
                                    page_input_text.set(total_pages.to_string());
                                },
                                "末页 ⏭"
                            }
                        }
                        div { class: "pagination-options",
                            // In-File Text Search Button & Collapsible Input
                            div { class: "file-search-wrap",
                                button {
                                    class: if *show_file_search.read() || !file_search_query.read().is_empty() {
                                        "file-search-toggle-btn active"
                                    } else {
                                        "file-search-toggle-btn"
                                    },
                                    title: "搜索当前会话文件内部文字（代码/输出/对话等，点击展开/收起）",
                                    onclick: move |_| {
                                        show_file_search.toggle();
                                    },
                                    if !file_search_query.read().is_empty() {
                                        "🔍 搜文件(已筛选)"
                                    } else {
                                        "🔍 搜文件内容"
                                    }
                                }
                                if *show_file_search.read() {
                                    div { class: "file-search-input-box",
                                        input {
                                            class: "file-search-input",
                                            placeholder: "搜索文件内部文字...",
                                            value: "{file_search_query}",
                                            oninput: move |e| {
                                                file_search_query.set(e.value());
                                                current_page.set(1);
                                                page_input_text.set("1".to_string());
                                            },
                                        }
                                        if !file_search_query.read().is_empty() {
                                            button {
                                                class: "file-search-clear-btn",
                                                title: "清空搜索",
                                                onclick: move |_| {
                                                    file_search_query.set(String::new());
                                                    current_page.set(1);
                                                    page_input_text.set("1".to_string());
                                                },
                                                "✕"
                                            }
                                        }
                                    }
                                }
                            }
                            button {
                                class: if is_main_single { "page-mode-btn active" } else { "page-mode-btn" },
                                title: if is_main_single { "切换回分页浏览" } else { "单页显示当前会话全部步骤" },
                                onclick: move |_| {
                                    let cur = *main_single_page.read();
                                    main_single_page.set(!cur);
                                    current_page.set(1);
                                    page_input_text.set("1".to_string());
                                },
                                if is_main_single {
                                    "📑 分页显示"
                                } else {
                                    "📄 单页显示"
                                }
                            }
                            if !is_main_single {
                                span { class: "page-size-label", "每页:" }
                                select {
                                    class: "page-size-select",
                                    value: "{page_size_num}",
                                    onchange: move |e| {
                                        if let Ok(size) = e.value().parse::<usize>() {
                                            page_size.set(size);
                                            current_page.set(1);
                                            page_input_text.set("1".to_string());
                                        }
                                    },
                                    option { value: "30", "30 步" }
                                    option { value: "50", "50 步" }
                                    option { value: "100", "100 步" }
                                    option { value: "200", "200 步" }
                                }
                            }
                        }
                    }

                    // Timeline Feed
                    div { class: "timeline-feed",
                        if page_steps.is_empty() {
                            div { class: "empty-search-hint",
                                div { class: "empty-icon", "🔍" }
                                if !file_search_query.read().is_empty() {
                                    p { "当前文件中未搜索到包含 \"{file_search_query}\" 的内容" }
                                    button {
                                        class: "clear-search-link-btn",
                                        onclick: move |_| {
                                            file_search_query.set(String::new());
                                            current_page.set(1);
                                            page_input_text.set("1".to_string());
                                        },
                                        "清空搜索条件"
                                    }
                                } else {
                                    p { "当前筛选条件下暂无步骤记录" }
                                }
                            }
                        }
                        // Steps Cards
                        for step in page_steps.iter() {
                            {
                                let step_id = step.id;
                                let is_raw_open = expanded_raw.read().contains(&step_id);
                                let is_thinking_open = expanded_thinking.read().contains(&step_id);
                                rsx! {
                                    div { class: "step-card", key: "{step_id}",
                                        // Header
                                        div { class: "step-header",
                                            div { class: "step-index-badge", "#{step.step_index}" }
                                            span { class: "type-tag type-{step.step_type.to_lowercase()}", "{step.step_type}" }
                                            span { class: "source-label", "来源: {step.source}" }
                                            span { class: "timestamp-label", "{step.created_at}" }
                                            div { class: "spacer" }
                                            button {
                                                class: "raw-json-btn",
                                                onclick: move |_| {
                                                    let mut set = expanded_raw.write();
                                                    if set.contains(&step_id) {
                                                        set.remove(&step_id);
                                                    } else {
                                                        set.insert(step_id);
                                                    }
                                                },
                                                if is_raw_open { "隐藏原始 JSON" } else { "{{ }} 原始 JSON" }
                                            }
                                        }

                                        // Step Content Blocks
                                        // 1. User Input
                                        if step.step_type == "USER_INPUT" {
                                            div { class: "user-input-box",
                                                div { class: "box-label", "👤 用户提问与输入" }
                                                div { class: "user-prompt-text",
                                                    "{step.content.as_deref().unwrap_or(\"\")}"
                                                }
                                            }
                                        }

                                        // 2. Thinking Chain
                                        if let Some(ref think) = step.thinking {
                                            div { class: "thinking-box",
                                                div {
                                                    class: "thinking-header",
                                                    onclick: move |_| {
                                                        let mut set = expanded_thinking.write();
                                                        if set.contains(&step_id) {
                                                            set.remove(&step_id);
                                                        } else {
                                                            set.insert(step_id);
                                                        }
                                                    },
                                                    span { class: "thinking-icon", "🧠" }
                                                    span { class: "thinking-title", "模型深度思考过程 (Thinking Process)" }
                                                    span { class: "toggle-hint", if is_thinking_open { "点击折叠 ▲" } else { "点击展开全文 ▼" } }
                                                }
                                                if is_thinking_open {
                                                    div { class: "thinking-body", "{think}" }
                                                } else {
                                                    div { class: "thinking-preview",
                                                        "{think.lines().take(2).collect::<Vec<_>>().join(\" \")}..."
                                                    }
                                                }
                                            }
                                        }

                                        // 3. Tool Calls (What tools did AGY call?)
                                        if !step.tool_calls.is_empty() {
                                            div { class: "tools-container",
                                                div { class: "box-label-tool", "🛠️ 发起工具调用 (Tool Invocations)" }
                                                for tool in step.tool_calls.iter() {
                                                    div { class: "tool-call-row",
                                                        div { class: "tool-name-badge", "{tool.name}" }
                                                        div { class: "tool-details",
                                                            {render_tool_detail(&tool.name, &tool.args)}
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // 4. Tool Execution Results (Run Command output, view file output, etc.)
                                        if step.step_type == "RUN_COMMAND" || step.step_type == "VIEW_FILE" || step.step_type == "REPLACE_FILE_CONTENT" || step.step_type == "WRITE_TO_FILE" {
                                            div { class: "tool-result-box",
                                                div { class: "result-top-bar",
                                                    span { class: "result-label", "📄 工具执行输出与结果" }
                                                    if let Some(code) = step.exit_code {
                                                        span {
                                                            class: if code == 0 { "exit-code-success" } else { "exit-code-fail" },
                                                            "Exit Code: {code}"
                                                        }
                                                    }
                                                }
                                                pre { class: "result-content",
                                                    "{safe_truncate(step.content.as_deref().unwrap_or(\"(无文本输出)\"), 30000)}"
                                                }
                                            }
                                        }

                                        // 5. Final Planner Response
                                        if step.step_type == "PLANNER_RESPONSE" && step.content.is_some() && step.tool_calls.is_empty() {
                                            div { class: "planner-response-box",
                                                div { class: "box-label-model", "🤖 AI 最终回复" }
                                                div { class: "model-reply-text",
                                                    "{step.content.as_deref().unwrap_or(\"\")}"
                                                }
                                            }
                                        }

                                        // Raw JSON Accordion
                                        if is_raw_open {
                                            div { class: "raw-json-viewer",
                                                div { class: "raw-json-bar",
                                                    span { "底层记录 JSONL (格式化视图)" }
                                                }
                                                pre { class: "json-code", "{format_pretty_json(&step.raw_json)}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Bottom Pagination Bar if multiple pages
                        if total_pages > 1 {
                            div { class: "bottom-pagination-bar",
                                div { class: "pagination-info",
                                    span { "第 {cur_page} / {total_pages} 页" }
                                }
                                div { class: "pagination-nav",
                                    button {
                                        class: "page-btn",
                                        disabled: cur_page <= 1,
                                        onclick: move |_| {
                                            current_page.set(1);
                                            page_input_text.set("1".to_string());
                                        },
                                        "⏮ 首页"
                                    }
                                    button {
                                        class: "page-btn",
                                        disabled: cur_page <= 1,
                                        onclick: move |_| {
                                            let p = cur_page.saturating_sub(1).max(1);
                                            current_page.set(p);
                                            page_input_text.set(p.to_string());
                                        },
                                        "◀ 上一页"
                                    }
                                    span { class: "page-indicator", "{cur_page} / {total_pages}" }
                                    button {
                                        class: "page-btn",
                                        disabled: cur_page >= total_pages,
                                        onclick: move |_| {
                                            let p = (cur_page + 1).min(total_pages);
                                            current_page.set(p);
                                            page_input_text.set(p.to_string());
                                        },
                                        "下一页 ▶"
                                    }
                                    button {
                                        class: "page-btn",
                                        disabled: cur_page >= total_pages,
                                        onclick: move |_| {
                                            current_page.set(total_pages);
                                            page_input_text.set(total_pages.to_string());
                                        },
                                        "末页 ⏭"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "empty-view",
                        if *is_sidebar_collapsed.read() {
                            button {
                                class: "expand-sidebar-btn empty-expand-btn",
                                title: "展开左侧栏",
                                onclick: move |_| {
                                    is_sidebar_collapsed.set(false);
                                },
                                "▶ 展开侧栏"
                            }
                        }
                        div { class: "empty-brand-logo",
                            img {
                                src: "{icon_data::APP_ICON_DATA_URL}",
                                alt: "Antigravity",
                            }
                        }
                        h3 { "未选择任何会话" }
                        p { "请在左侧侧边栏中选择一个 Antigravity 对话以查看全流程透明记录。" }
                    }
                }
            }

            // Modal: Confirm Delete Conversation
            if let Some(target) = delete_modal_conv.read().as_ref() {
                div {
                    class: "modal-overlay",
                    onclick: move |_| {
                        delete_error.set(None);
                        delete_modal_conv.set(None);
                    },
                    div {
                        class: "modal-card",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "modal-header",
                            span { class: "modal-icon", "⚠️" }
                            h4 { "确认删除该会话？" }
                        }
                        div { class: "modal-body",
                            p { class: "modal-desc", "该操作将彻底删除此会话在本地磁盘上的全部日志及相关数据，删除后无法恢复！" }
                            div { class: "modal-meta-box",
                                div { class: "modal-meta-row",
                                    span { class: "modal-meta-key", "会话 ID:" }
                                    span { class: "modal-meta-val uuid-font", "{target.id}" }
                                }
                                div { class: "modal-meta-row",
                                    span { class: "modal-meta-key", "数据源:" }
                                    span { class: "modal-meta-val", "{target.source_name}" }
                                }
                                div { class: "modal-meta-row",
                                    span { class: "modal-meta-key", "内容预览:" }
                                    span { class: "modal-meta-val", "{target.title_preview}" }
                                }
                                div { class: "modal-meta-row",
                                    span { class: "modal-meta-key", "磁盘路径:" }
                                    span { class: "modal-meta-val path-font", "{target.path.display()}" }
                                }
                            }
                            if let Some(err_msg) = delete_error.read().as_ref() {
                                div { class: "modal-error-box", "❌ {err_msg}" }
                            }
                        }
                        div { class: "modal-footer",
                            button {
                                class: "modal-btn-cancel",
                                onclick: move |_| {
                                    delete_error.set(None);
                                    delete_modal_conv.set(None);
                                },
                                "取消"
                            }
                            button {
                                class: "modal-btn-confirm-delete",
                                onclick: {
                                    let target_path = target.path.clone();
                                    let target_id = target.id.clone();
                                    move |_| {
                                        if let Err(e) = scanner::delete_conversation_folder(&target_path) {
                                            delete_error.set(Some(e));
                                        } else {
                                            delete_error.set(None);
                                            delete_modal_conv.set(None);
                                            let new_list = scan_conversations();
                                            if selected_id.read().as_ref() == Some(&target_id) {
                                                selected_id.set(new_list.first().map(|c| c.id.clone()));
                                            }
                                            let max_p = new_list.len().div_ceil(sidebar_page_size).max(1);
                                            conversations.set(new_list);
                                            current_page.set(1);
                                            page_input_text.set("1".to_string());
                                            let clamped_sp = (*sidebar_page.read()).min(max_p);
                                            sidebar_page.set(clamped_sp);
                                            sidebar_page_input.set(clamped_sp.to_string());
                                            expanded_raw.write().clear();
                                            expanded_thinking.write().clear();
                                        }
                                    }
                                },
                                "彻底删除"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_tool_detail(name: &str, args: &serde_json::Value) -> Element {
    let clean_str = |val: Option<&serde_json::Value>| -> String {
        val.map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
    };

    match name {
        "view_file" => {
            let path = clean_str(args.get("AbsolutePath"));
            let action = clean_str(args.get("toolAction"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge file-badge", "读取文件" }
                    code { class: "target-val", "{path}" }
                    if !action.is_empty() {
                        span { class: "action-desc", "({action})" }
                    }
                }
            }
        }
        "run_command" => {
            let cmd = clean_str(args.get("CommandLine"));
            let cwd = clean_str(args.get("Cwd"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge cmd-badge", "执行命令" }
                    code { class: "target-val cmd-val", "{cmd}" }
                    if !cwd.is_empty() {
                        span { class: "cwd-desc", "目录: {cwd}" }
                    }
                }
            }
        }
        "replace_file_content" | "multi_replace_file_content" => {
            let file = clean_str(args.get("TargetFile"));
            let desc = clean_str(args.get("Description"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge edit-badge", "修改文件" }
                    code { class: "target-val", "{file}" }
                    if !desc.is_empty() {
                        span { class: "action-desc", "{desc}" }
                    }
                }
            }
        }
        "write_to_file" => {
            let file = clean_str(args.get("TargetFile"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge write-badge", "新建/写入文件" }
                    code { class: "target-val", "{file}" }
                }
            }
        }
        "list_dir" => {
            let dir = clean_str(args.get("DirectoryPath"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge dir-badge", "遍历目录" }
                    code { class: "target-val", "{dir}" }
                }
            }
        }
        "grep_search" => {
            let query = clean_str(args.get("Query"));
            let path = clean_str(args.get("SearchPath"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge search-badge", "全文搜索" }
                    code { class: "target-val", "\"{query}\"" }
                    span { class: "cwd-desc", "在 {path}" }
                }
            }
        }
        "ask_question" => {
            let q_preview = if let Some(questions) = args.get("questions").and_then(|q| q.as_array()) {
                questions
                    .iter()
                    .filter_map(|item| item.get("question").and_then(|q| q.as_str()))
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                clean_str(args.get("question"))
            };
            let preview = safe_truncate(&q_preview, 100);
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge ask-badge", "交互提问" }
                    code { class: "target-val", "{preview}" }
                }
            }
        }
        "manage_task" => {
            let action = clean_str(args.get("Action"));
            let task_id = clean_str(args.get("TaskId"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge task-badge", "任务管理" }
                    code { class: "target-val", "{action} [{task_id}]" }
                }
            }
        }
        "schedule" => {
            let dur = clean_str(args.get("DurationSeconds"));
            let cron = clean_str(args.get("CronExpression"));
            let prompt = clean_str(args.get("Prompt"));
            let prompt_preview = safe_truncate(&prompt, 60);
            let time_info = if !dur.is_empty() {
                format!("{dur}s")
            } else if !cron.is_empty() {
                cron
            } else {
                "-".to_string()
            };
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge schedule-badge", "定时调度" }
                    code { class: "target-val", "{time_info}" }
                    if !prompt_preview.is_empty() {
                        span { class: "action-desc", "{prompt_preview}" }
                    }
                }
            }
        }
        "search_web" => {
            let query = clean_str(args.get("query"));
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge web-badge", "网络搜索" }
                    code { class: "target-val", "\"{query}\"" }
                }
            }
        }
        "read_url_content" | "read_browser_page" => {
            let url = clean_str(args.get("Url"));
            let url_preview = safe_truncate(&url, 80);
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge web-badge", "抓取网页" }
                    code { class: "target-val", "{url_preview}" }
                }
            }
        }
        "generate_image" => {
            let name = clean_str(args.get("ImageName"));
            let prompt = clean_str(args.get("Prompt"));
            let prompt_preview = safe_truncate(&prompt, 60);
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge image-badge", "图像生成" }
                    code { class: "target-val", "{name}" }
                    if !prompt_preview.is_empty() {
                        span { class: "action-desc", "{prompt_preview}" }
                    }
                }
            }
        }
        _ => {
            let args_preview = args.to_string();
            let preview = safe_truncate(&args_preview, 100);
            rsx! {
                div { class: "tool-parsed-item",
                    span { class: "key-badge default-badge", "参数" }
                    code { class: "target-val", "{preview}" }
                }
            }
        }
    }
}
