# Memory Optimization

R Teams giảm **200-280MB RAM** so với WebView2 mặc định bằng cách tắt các Chromium features Teams không cần.

## Profiles

| Profile       | RAM Saved | Trade-off |
|---------------|-----------|-----------|
| `safe`        | ~70MB     | Spectre protection ON, cache ON |
| `balanced` *(default)* | ~250MB | Spectre protection OFF, cache OFF |
| `aggressive`  | ~350MB    | Có thể crash nếu Teams cần >512MB heap hoặc >2 renderer |

## CLI Usage

```bash
rust_teams.exe --memory-profile safe
rust_teams.exe --memory-profile balanced
rust_teams.exe --memory-profile aggressive
rust_teams.exe --memory-profile off
```

## Config File

File: `%APPDATA%\rust_teams\config.toml`

```toml
[memory_optimization]
enabled = true
disable_gpu = true
disable_background_networking = true
disable_breakpad = true
disable_sync = true
disable_translate = true
disable_extensions = true
disable_component_update = true
disable_domain_reliability = true
disable_back_forward_cache = true
disable_site_isolation = true
renderer_process_limit = 0   # 0 = unlimited
js_max_old_space_mb = 0      # 0 = unlimited
```

## Flag Reference

| Flag | Saved | What it does | Risk |
|------|-------|--------------|------|
| `--disable-gpu` | ~50MB | Tắt GPU acceleration, dùng CPU render | Scroll chậm hơn 1 chút |
| `--disable-background-networking` | ~10MB | Tắt telemetry, safe-browsing ping | Không ảnh hưởng chức năng |
| `--disable-breakpad` | ~5MB | Tắt crash reporter | Không gửi crash data |
| `--disable-sync` | ~5MB | Tắt Chrome sync | Teams không sync gì qua Chromium |
| `--disable-translate` | ~5MB | Tắt translate UI | Không ảnh hưởng |
| `--disable-extensions` | ~10MB | Tắt extension loading | Teams không dùng extension |
| `--disable-component-update` | ~5MB | Không tự update WebView2 | Phải update thủ công |
| `--disable-domain-reliability` | ~2MB | Tắt telemetry | Không ảnh hưởng |
| `--disable-features=BackForwardCache` | ~30MB | Tắt BFCache | Back/forward chậm hơn |
| `--disable-features=IsolateOrigins,site-per-process` | ~80-120MB | Tắt site isolation (Spectre mitigation) | Cross-origin iframe có thể share process |

## JS Optimizations (từ performance.rs)

Ngoài browser flags, R Teams còn inject script tối ưu JavaScript:

1. **Preconnect hints** — DNS/TLS warmup cho Teams domains
2. **Visibility pause** — Khi user chuyển tab, pause non-essential timers
3. **Idle GC hint** — Sau 30s không tương tác, hint browser giải phóng cached images
4. **Content visibility** — Render chậm cho off-screen chat lists (CSS `content-visibility: auto`)

## Trade-offs

**Spectre protection OFF (Balanced):** Chromium mặc định tách mỗi cross-origin iframe vào process riêng để chống Spectre. Tắt → các iframe Teams (auth.microsoft.com, graph.microsoft.com, v.v.) share process. Rủi ro:
- Nếu 1 iframe crash → crash cả tab
- Cross-origin timing attack có thể leak data (nhưng attacker phải có JS execution trên 1 origin)

Vì Teams UI là trusted, đây là trade-off chấp nhận được cho desktop app cá nhân.

## Measured Impact

Trên Windows 11, Teams 2.x với 5 chat threads + 1 tab calls:

| Mode | RSS Memory | Working Set |
|------|-----------|-------------|
| WebView2 default | ~850MB | ~620MB |
| R Teams Balanced | ~600MB | ~370MB |
| R Teams Aggressive | ~520MB | ~310MB |

*Tested with WebView2 Runtime 130.0.2849.80, không có video call đang chạy.*

## References

- [Chromium command-line switches](https://peter.sh/experiments/chromium-command-line-switches/)
- [WebView2 spec - additional browser args](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/environment-options)
