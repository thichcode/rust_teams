//! Memory optimization module
//! Translates `MemoryOptimization` config into Chromium / WebView2
//! browser arguments and provides CLI profile override.
//!
//! ## Profiles
//!
//! - **safe**       — chỉ flags an toàn (~70MB saved)
//! - **balanced**   — default, khoảng 200-280MB saved, sacrifice Spectre
//! - **aggressive** — balanced + render limit + JS heap limit (có thể crash nếu Teams cần nhiều RAM)

use crate::app::MemoryOptimization;

/// CLI override memory profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProfile {
    Safe,
    Balanced,
    Aggressive,
    Disabled,
}

impl MemoryProfile {
    /// Parse profile name từ CLI argument
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "safe" => Some(Self::Safe),
            "balanced" => Some(Self::Balanced),
            "aggressive" => Some(Self::Aggressive),
            "off" | "disabled" | "none" => Some(Self::Disabled),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
            Self::Disabled => "disabled",
        }
    }

    /// Áp dụng profile vào config, ghi đè các flag
    pub fn apply_to(&self, cfg: &mut MemoryOptimization) {
        match self {
            Self::Disabled => {
                cfg.enabled = false;
            }
            Self::Safe => {
                cfg.enabled = true;
                cfg.disable_gpu = true;
                cfg.disable_background_networking = true;
                cfg.disable_breakpad = true;
                cfg.disable_sync = true;
                cfg.disable_translate = true;
                cfg.disable_extensions = true;
                cfg.disable_component_update = true;
                cfg.disable_domain_reliability = true;
                // Conservative — không tắt cache, site isolation
                cfg.disable_back_forward_cache = false;
                cfg.disable_site_isolation = false;
                cfg.renderer_process_limit = 0;
                cfg.js_max_old_space_mb = 0;
            }
            Self::Balanced => {
                cfg.enabled = true;
                cfg.disable_gpu = true;
                cfg.disable_background_networking = true;
                cfg.disable_breakpad = true;
                cfg.disable_sync = true;
                cfg.disable_translate = true;
                cfg.disable_extensions = true;
                cfg.disable_component_update = true;
                cfg.disable_domain_reliability = true;
                cfg.disable_back_forward_cache = true;
                cfg.disable_site_isolation = true;
                // Không giới hạn process / heap (giữ an toàn)
                cfg.renderer_process_limit = 0;
                cfg.js_max_old_space_mb = 0;
            }
            Self::Aggressive => {
                cfg.enabled = true;
                cfg.disable_gpu = true;
                cfg.disable_background_networking = true;
                cfg.disable_breakpad = true;
                cfg.disable_sync = true;
                cfg.disable_translate = true;
                cfg.disable_extensions = true;
                cfg.disable_component_update = true;
                cfg.disable_domain_reliability = true;
                cfg.disable_back_forward_cache = true;
                cfg.disable_site_isolation = true;
                cfg.renderer_process_limit = 2;
                cfg.js_max_old_space_mb = 512;
            }
        }
    }
}

/// Build chuỗi browser args từ MemoryOptimization config
/// Truyền vào `WebViewBuilder::with_additional_browser_args()` (Windows)
pub fn build_browser_args(cfg: &MemoryOptimization) -> String {
    if !cfg.enabled {
        return String::new();
    }

    let mut flags: Vec<String> = vec!["--no-first-run".into(), "--no-default-browser-check".into()];

    if cfg.disable_gpu {
        flags.push("--disable-gpu".into());
    }
    if cfg.disable_background_networking {
        flags.push("--disable-background-networking".into());
    }
    if cfg.disable_breakpad {
        flags.push("--disable-breakpad".into());
    }
    if cfg.disable_sync {
        flags.push("--disable-sync".into());
    }
    if cfg.disable_translate {
        flags.push("--disable-translate".into());
    }
    if cfg.disable_extensions {
        flags.push("--disable-extensions".into());
    }
    if cfg.disable_component_update {
        flags.push("--disable-component-update".into());
    }
    if cfg.disable_domain_reliability {
        flags.push("--disable-domain-reliability".into());
    }

    let mut disable_features: Vec<&str> = Vec::new();
    if cfg.disable_back_forward_cache {
        disable_features.push("BackForwardCache");
    }
    if cfg.disable_site_isolation {
        disable_features.push("IsolateOrigins");
        disable_features.push("site-per-process");
    }
    if !disable_features.is_empty() {
        flags.push(format!("--disable-features={}", disable_features.join(",")));
    }

    if cfg.renderer_process_limit > 0 {
        flags.push(format!(
            "--renderer-process-limit={}",
            cfg.renderer_process_limit
        ));
    }
    if cfg.js_max_old_space_mb > 0 {
        flags.push(format!(
            "--js-flags=--max-old-space-size={}",
            cfg.js_max_old_space_mb
        ));
    }

    flags.join(" ")
}

/// Detect which profile best matches the current config.
pub fn detect_profile(cfg: &MemoryOptimization) -> &'static str {
    if !cfg.enabled {
        return "OFF";
    }
    if cfg.renderer_process_limit > 0 || cfg.js_max_old_space_mb > 0 {
        "Aggressive"
    } else if cfg.disable_site_isolation && cfg.disable_back_forward_cache {
        "Balanced"
    } else if !cfg.disable_site_isolation && !cfg.disable_back_forward_cache {
        "Safe"
    } else {
        "Custom"
    }
}

/// Log tóm tắt các flag sẽ áp dụng (gọi sau `build_browser_args`)
pub fn log_summary(cfg: &MemoryOptimization) {
    if !cfg.enabled {
        log::info!("Memory optimization: OFF (WebView2 defaults)");
        return;
    }

    let profile = detect_profile(cfg);
    log::info!("Memory optimization: ON ({profile})");
    log::info!(
        "  GPU:                  {}",
        if cfg.disable_gpu {
            "disabled"
        } else {
            "enabled"
        }
    );
    log::info!(
        "  Background networking:{}",
        if cfg.disable_background_networking {
            "off"
        } else {
            "on"
        }
    );
    log::info!(
        "  Breakpad:             {}",
        if cfg.disable_breakpad { "off" } else { "on" }
    );
    log::info!(
        "  Sync:                 {}",
        if cfg.disable_sync { "off" } else { "on" }
    );
    log::info!(
        "  Translate:            {}",
        if cfg.disable_translate { "off" } else { "on" }
    );
    log::info!(
        "  Extensions:           {}",
        if cfg.disable_extensions { "off" } else { "on" }
    );
    log::info!(
        "  Component update:     {}",
        if cfg.disable_component_update {
            "off"
        } else {
            "on"
        }
    );
    log::info!(
        "  Domain reliability:   {}",
        if cfg.disable_domain_reliability {
            "off"
        } else {
            "on"
        }
    );
    log::info!(
        "  BackForwardCache:     {}",
        if cfg.disable_back_forward_cache {
            "off"
        } else {
            "on"
        }
    );
    log::info!(
        "  Site isolation:       {}",
        if cfg.disable_site_isolation {
            "off (Spectre OFF)"
        } else {
            "on"
        }
    );
    if cfg.renderer_process_limit > 0 {
        log::info!("  Renderer process limit: {}", cfg.renderer_process_limit);
    }
    if cfg.js_max_old_space_mb > 0 {
        log::info!("  V8 max old space:       {}MB", cfg.js_max_old_space_mb);
    }
}

/// Parse CLI args trước khi load config
/// Returns Some(profile) nếu --memory-profile được chỉ định
pub fn parse_cli_profile(args: &[String]) -> Option<MemoryProfile> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--memory-profile" {
            if let Some(val) = args.get(i + 1) {
                return MemoryProfile::from_str(val);
            }
        } else if let Some(rest) = arg.strip_prefix("--memory-profile=") {
            return MemoryProfile::from_str(rest);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_balanced() {
        let cfg = MemoryOptimization::default();
        let args = build_browser_args(&cfg);
        assert!(args.contains("--disable-gpu"));
        assert!(args.contains("IsolateOrigins"));
        assert!(args.contains("site-per-process"));
        assert!(args.contains("BackForwardCache"));
        assert!(args.contains("--no-first-run"));
    }

    #[test]
    fn test_build_args_disabled() {
        let cfg = MemoryOptimization {
            enabled: false,
            ..Default::default()
        };
        let args = build_browser_args(&cfg);
        assert!(args.is_empty());
    }

    #[test]
    fn test_profile_apply() {
        let mut cfg = MemoryOptimization::default();
        MemoryProfile::Safe.apply_to(&mut cfg);
        assert!(!cfg.disable_site_isolation);

        MemoryProfile::Aggressive.apply_to(&mut cfg);
        assert_eq!(cfg.renderer_process_limit, 2);
        assert_eq!(cfg.js_max_old_space_mb, 512);
    }
}
