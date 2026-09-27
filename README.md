# Antigravity Historical Dialogue

A high-performance, transparent dialogue auditing and trajectory monitoring tool for **Google Antigravity** (including CLI, IDE extension, and Antigravity 2.0 desktop). Built with **Rust + Dioxus 0.7**.

---

## 🌟 Key Features

### 1. 100% Transparent Trajectory Auditing
- 👤 **User Request**: Captures raw user inputs, surrounding prompt context, and exact timestamps.
- 🧠 **Deep Thinking Chain**: Faithfully presents the internal reasoning process of the model prior to formulating answers (with one-click collapse/expand).
- 🛠️ **Accurate Tool Call Tracking**:
  - `view_file`: Exact file paths read and inspected.
  - `run_command`: Exact terminal commands executed and working directories.
  - `replace_file_content` / `write_to_file`: File modifications and creations with target paths and diff descriptions.
  - `list_dir`, `grep_search`, and all other system tools.
- 📄 **Tool Execution Outputs**: View stdout/stderr slices, file chunks, and process exit codes.
- 🤖 **Model Responses**: Clean markdown presentation of the final answers.
- 🔍 **Raw JSONL Inspector**: Every step provides a `[ { } Raw JSON ]` button to inspect the underlying protocol messages directly.

### 2. Triple Data Source Auto-Discovery
Automatically scans and detects conversation records across all three Google Antigravity storage locations:
- `~/.gemini/antigravity/brain` (Antigravity 2.0 Standalone Desktop sessions)
- `~/.gemini/antigravity-ide/brain` (Antigravity IDE Plugin sessions)
- `~/.gemini/antigravity-cli/brain` (Antigravity CLI sessions)

### 3. Massive & Long Conversation Optimization
- ⚡ **Step Pagination**: Easily handle massive sessions (10,000+ steps and dozens of megabytes) with 30 / 50 / 100 / 200 steps-per-page options.
- 🎯 **Direct Page Jumping**: Pinned top and bottom pagination bars with direct numeric input and Enter-key jumping.
- 🛡️ **Strict Unique Identification**: Eliminates Dioxus diff reconciliation crashes on duplicate step indexes from sub-agents.
- 🔒 **UTF-8 Character Boundary Safety**: Prevents slicing panics on multi-byte characters and emoji in tool preview snippets.
- 🚀 **Lazy Formatting**: Parses and formats raw JSON on demand for 10x faster startup scanning.

### 4. Session Management & Smart Backup
- 💾 **Dual-Mode Backup Engine**:
  - **Normal Backup**: Incremental backup copying only sessions added or modified since the last backup timestamp recorded in `backup_info.json`.
  - **Full Check Backup**: Comprehensive rescan and full synchronization of all sessions to the `backups/` directory.
  - **Feedback Notifications**: Card popups with last backup metadata and a one-click **"📂 Open Backup Folder"** button.
- 🗑️ **Safe Session Deletion**: Delete corrupted or unwanted sessions from the sidebar or main header with physical disk cleanup and safety confirmation modals.
- ↔️ **Draggable Resizer**: Sidebar width is smoothly adjustable with drag-and-drop and remembers user preferences via local storage.

### 5. Internationalization & Custom Brand Icon
- 🌐 **Bilingual (English / Chinese)**: Instant runtime toggle button in the header toolbar.
  - **Release builds (`--release`)** default to **English**.
  - **Debug builds** default to **Chinese**.
- 🎨 **Official Antigravity Brand Identity**:
  - Custom SVG icon featuring a 12×12 space-time coordinate grid and a 4-layer Gaussian wave gradient (Emerald Green, Coral Orange, Cyan Radiance, and Royal Blue).
  - Native multi-resolution `icon.ico` embedded directly into the Windows `.exe` binary resource.
  - Replaces default window title bar and taskbar icons.

---

## 🚀 Usage Modes

### Mode 1: Standalone Desktop Application

#### Option A: Run the Precompiled Binary
Simply double-click:
```text
antigravity-historical-dialogue.exe
```

#### Option B: Run from Source
```bash
cargo run --release
```

---

### Mode 2: Turnkey Embeddable Dioxus Library Component

This project is packaged with a dual-target architecture (`[lib]` + `[[bin]]`). You can embed the entire dialogue viewer into any Dioxus desktop or web application with just a single line of code!

#### Step 1: Add Dependency (`Cargo.toml`)
```toml
[dependencies]
antigravity_historical_dialogue = { path = "path/to/Antigravity Historical Dialogue" }
```

#### Step 2: Use in Your Dioxus App

##### Approach A: Turnkey Tab Button with Popup Overlay (Zero Configuration)
Renders a neat tab button that automatically opens the full historical viewer in an overlay window with a built-in close button:

```rust
use antigravity_historical_dialogue::HistoricalDialogueTab;
use dioxus::prelude::*;

fn Header() -> Element {
    rsx! {
        div { class: "nav-bar",
            span { "My App" }

            // Turnkey tab button: click to open full history audit dialog!
            HistoricalDialogueTab {
                label: "History Audit", // Optional custom label
                icon: "📜",             // Optional custom icon
            }
        }
    }
}
```

##### Approach B: Direct Embedded Tab Content Area
Seamlessly embed the viewer inside your own tab navigation or panel system. It automatically expands to fill the parent container's dimensions:

```rust
use antigravity_historical_dialogue::{HistoricalDialogueViewer, Language};
use dioxus::prelude::*;

fn App() -> Element {
    let mut active_tab = use_signal(|| "dashboard");

    rsx! {
        div { class: "tabs-header",
            button { onclick: move |_| active_tab.set("dashboard"), "Dashboard" }
            button { onclick: move |_| active_tab.set("history"), "📜 History" }
        }

        div { class: "tabs-content",
            if *active_tab.read() == "history" {
                HistoricalDialogueViewer {
                    default_lang: Some(Language::En), // Optional language override
                    on_close: move |_| active_tab.set("dashboard"), // Optional close callback
                }
            }
        }
    }
}
```

#### Component Props Reference (`HistoricalDialogueProps`)

| Prop | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `default_lang` | `Option<Language>` | `None` (Debug: Zh, Release: En) | Initial UI language (`Language::En` or `Language::Zh`). |
| `class` | `String` | `""` | Extra CSS class names for the root container. |
| `on_close` | `Option<EventHandler<()>>` | `None` | Callback invoked when the user clicks the close `✕` button. |

---

## 📁 Project Structure

```text
├── Cargo.toml                          # Dual-target package configuration ([lib] + [[bin]])
├── build.rs                            # Windows PE resource icon compiler (winres)
├── icon.svg                            # Vector Antigravity brand icon
├── icon.png                            # 512x512 RGBA high-resolution image
├── icon.ico                            # Multi-resolution Windows executable icon (16~256px)
├── antigravity-historical-dialogue.exe # Native Windows release binary
├── src/
│   ├── lib.rs                          # Library entrypoint & reusable Dioxus components
│   ├── main.rs                         # Standalone desktop executable launcher
│   ├── models.rs                       # Conversation, Step, and Tool call data definitions
│   ├── scanner.rs                      # Disk scanning & resilient JSONL parsing engine
│   ├── backup.rs                       # Smart incremental & full sync backup operations
│   ├── i18n.rs                         # Bilingual localization module (En / Zh)
│   ├── icon_data.rs                    # Embedded Base64 SVG data URL asset
│   └── style.css                       # Modern light theme & responsive layout stylesheet
```

---

## 🔒 Code Quality & Safety Guarantees

This project enforces strict compiler and linter settings:
- `unsafe_code = "forbid"`: Zero unsafe code allowed.
- `warnings = "deny"`: Zero compiler warnings allowed across both library and binary builds.
- Strict Clippy passes with all correctness, performance, and complexity lints enabled.
