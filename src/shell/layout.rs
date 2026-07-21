use dioxus::prelude::*;

use super::bridge::{ShellCommand, ShellState, TeamsStatus};

#[component]
pub fn ShellApp(state: ShellState, on_command: EventHandler<ShellCommand>) -> Element {
    let status_text = match &state.teams_status {
        TeamsStatus::Loading => "⏳ Loading...".to_string(),
        TeamsStatus::Ready => "✅ Connected".to_string(),
        TeamsStatus::Error(e) => format!("❌ {}", e),
    };
    let status_color = match &state.teams_status {
        TeamsStatus::Loading => "#f0ad4e",
        TeamsStatus::Ready => "#5cb85c",
        TeamsStatus::Error(_) => "#d9534f",
    };

    rsx! {
        div {
            style: "display:flex; height:100vh; font-family: 'Segoe UI', sans-serif; margin:0; background:#1b1b1f; color:#e0e0e0;",

            // Sidebar
            div {
                style: "width:240px; background:#202025; display:flex; flex-direction:column; border-right:1px solid #333; padding:12px 0;",

                // App logo
                div {
                    style: "padding:12px 16px; display:flex; align-items:center; gap:10px; margin-bottom:8px;",
                    span { style: "font-size:20px;", "🦀" }
                    span { style: "font-size:15px; font-weight:600; color:#c5c5d2;", "R Teams" }
                    span { style: "font-size:11px; color:#666; margin-left:auto;", "v{state.app_version}" }
                }

                // Status indicator
                div {
                    style: "padding:8px 16px; margin:4px 8px; border-radius:6px; font-size:12px; background:#2a2a30;",
                    div { style: "color:#888; margin-bottom:4px;", "Teams" }
                    div { style: "color:{status_color};", "{status_text}" }
                }

                // Profile section
                div {
                    style: "padding:8px 16px; margin:4px 8px; border-radius:6px; background:#2a2a30;",
                    div { style: "color:#888; font-size:11px; margin-bottom:4px;", "PROFILE" }
                    div { style: "color:#c5c5d2; font-size:13px;", "{state.current_profile}" }
                }

                // Memory profile
                div {
                    style: "padding:8px 16px; margin:4px 8px; border-radius:6px; background:#2a2a30;",
                    div { style: "color:#888; font-size:11px; margin-bottom:4px;", "MEMORY" }
                    div { style: "color:#c5c5d2; font-size:13px;", "{state.memory_profile}" }
                }

                // Unread badge
                if state.unread_count > 0 {
                    div {
                        style: "padding:8px 16px; margin:4px 8px; border-radius:6px; background:#2a2a30;",
                        div { style: "color:#888; font-size:11px; margin-bottom:4px;", "UNREAD" }
                        div { style: "color:#5bc0de; font-size:13px; font-weight:600;", "{state.unread_count} messages" }
                    }
                }

                // Spacer
                div { style: "flex:1;" }

                // Actions
                div {
                    style: "padding:0 8px;",
                    SidebarButton {
                        label: "🔄 Reload Teams".to_string(),
                        onclick: move |_| on_command.call(ShellCommand::ReloadTeams),
                    }
                    SidebarButton {
                        label: "⚙️ Settings".to_string(),
                        onclick: move |_| on_command.call(ShellCommand::OpenSettings),
                    }
                    SidebarButton {
                        label: "💾 Toggle Memory".to_string(),
                        onclick: move |_| on_command.call(ShellCommand::ToggleMemoryOptimization),
                    }
                    SidebarButton {
                        label: "❌ Quit".to_string(),
                        onclick: move |_| on_command.call(ShellCommand::Quit),
                    }
                }
            }

            // Main content area (placeholder for Teams)
            div {
                style: "flex:1; display:flex; align-items:center; justify-content:center; background:#1b1b1f;",
                div {
                    style: "text-align:center; color:#666;",
                    div { style: "font-size:48px; margin-bottom:16px;", "🦀" }
                    div { style: "font-size:18px; margin-bottom:8px;", "R Teams Dioxus Shell" }
                    div { style: "font-size:13px;", "Teams WebView will render here" }
                    div { style: "font-size:12px; margin-top:16px; color:#555;", "Profile: {state.current_profile}" }
                }
            }
        }
    }
}

#[component]
fn SidebarButton(label: String, onclick: EventHandler<()>) -> Element {
    rsx! {
        button {
            style: "display:block; width:100%; padding:10px 16px; border:none; background:transparent; color:#aaa; font-size:13px; text-align:left; cursor:pointer; border-radius:6px; margin:2px 0;",
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}
