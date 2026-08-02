//! Application configuration types
use serde::{Deserialize, Serialize};

pub use crate::ui::WindowSettings;

fn default_memory_config() -> MemoryOptimization {
    MemoryOptimization::default()
}

fn default_render_mode() -> WebkitRenderMode {
    WebkitRenderMode::default()
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebkitRenderMode {
    /// Default — no env vars. Requires GPU/compositor.
    Auto,
    /// Compat — disables GPU compositing + dmabuf; uses software rendering.
    #[default]
    Compat,
}

fn default_linux_backend() -> LinuxBackend {
    LinuxBackend::default()
}

/// Linux rendering backend choice.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LinuxBackend {
    /// Auto — use Chromium app-mode if a Chromium-based browser is installed,
    /// otherwise fall back to the embedded WebKitGTK webview.
    #[default]
    Auto,
    /// Always use the embedded WebKitGTK webview.
    Webkit,
    /// Always launch Teams in a Chromium-based browser app-mode window.
    Chromium,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub window_settings: WindowSettings,
    pub profiles: Vec<Profile>,
    pub current_profile_id: Option<String>,
    #[serde(default = "default_memory_config")]
    pub memory_optimization: MemoryOptimization,
    /// Path to preferred browser executable, or None for system default.
    #[serde(default)]
    pub browser_path: Option<String>,
    /// WebKitGTK rendering mode (Linux VDI compat).
    #[serde(default = "default_render_mode")]
    pub webkit_render_mode: WebkitRenderMode,
    /// Linux backend: embedded WebKit webview vs Chromium app-mode.
    #[serde(default = "default_linux_backend")]
    pub linux_backend: LinuxBackend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptimization {
    pub enabled: bool,
    pub max_cache_size_mb: u32,
    pub disable_gpu: bool,
    pub disable_animations: bool,
    pub idle_timeout_secs: u32,

    // ---- Chromium / WebView2 flags (Balanced profile defaults) ----
    /// Tắt background networking (ping, stats, safe-browsing telemetry)
    #[serde(default = "default_true")]
    pub disable_background_networking: bool,

    /// Tắt crash reporter
    #[serde(default = "default_true")]
    pub disable_breakpad: bool,

    /// Tắt Chrome sync
    #[serde(default = "default_true")]
    pub disable_sync: bool,

    /// Tắt Chromium translate UI
    #[serde(default = "default_true")]
    pub disable_translate: bool,

    /// Tắt extensions (Teams không cần)
    #[serde(default = "default_true")]
    pub disable_extensions: bool,

    /// Tắt tự động update WebView2 component
    #[serde(default = "default_true")]
    pub disable_component_update: bool,

    /// Tắt domain reliability telemetry
    #[serde(default = "default_true")]
    pub disable_domain_reliability: bool,

    /// Tắt BackForwardCache (~30MB nhưng back/forward chậm hơn)
    #[serde(default = "default_true")]
    pub disable_back_forward_cache: bool,

    /// Tắt site isolation (Spectre mitigation) — tiết kiệm 80-120MB
    #[serde(default = "default_true")]
    pub disable_site_isolation: bool,

    /// Giới hạn số renderer process (0 = unlimited)
    #[serde(default)]
    pub renderer_process_limit: u32,

    /// Giới hạn V8 heap MB (0 = unlimited)
    #[serde(default)]
    pub js_max_old_space_mb: u32,
}

fn default_true() -> bool {
    true
}

impl Default for MemoryOptimization {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_size_mb: 10,
            disable_gpu: true,
            disable_animations: true,
            idle_timeout_secs: 300,
            disable_background_networking: true,
            disable_breakpad: true,
            disable_sync: true,
            disable_translate: true,
            disable_extensions: true,
            disable_component_update: true,
            disable_domain_reliability: true,
            disable_back_forward_cache: true,
            disable_site_isolation: true,
            renderer_process_limit: 0,
            js_max_old_space_mb: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub teams_url: String,
    pub is_default: bool,
}
