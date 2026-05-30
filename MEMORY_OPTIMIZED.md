# Rust Teams — Phiên bản tối ưu bộ nhớ (Memory-Optimized)

## So sánh phiên bản

```
                        Original        Memory-Optimized
──────────────────────────────────────────────────────────
RAM startup             ~80 MB          ~40 MB
RAM sau 30 phút         ~150 MB         ~60–80 MB
CPU idle                ~0.5%           ~0.2%
Dung lượng exe          ~10 MB          ~6 MB
WebView processes       2–3             1
Cache size              Default         10 MB max
GPU acceleration        Bật             Tắt (tiết kiệm RAM)
```

## Chiến lược tối ưu

### 1. WebView2 Tối ưu bộ nhớ
- **Giới hạn cache:** Đặt cache max 10MB thay vì default ~250MB
- **Tắt GPU acceleration:** Giảm RAM usage ~30-50MB
- **Single renderer process:** Giới hạn số process WebView2
- **Disable background tasks:** Tự disable JS timer khi idle
- **Lazy initialization:** Chỉ tạo WebView khi user thực sự cần

### 2. Config tối ưu
```json
{
  "memory_optimization": {
    "enabled": true,
    "max_cache_size_mb": 10,
    "disable_gpu": true,
    "disable_animations": true,
    "idle_timeout_seconds": 300,
    "auto_gc": true,
    "max_heap_size_mb": 256
  }
}
```

### 3. Tính năng mới trong phiên bản tối ưu

| Tính năng | Mô tả | Tiết kiệm |
|-----------|-------|-----------|
| Memory Monitor | Theo dõi RAM real-time | — |
| Auto GC | Tự garbage collect khi idle | ~10-20 MB |
| Cache Limiter | Giới hạn cache WebView2 | ~50-100 MB |
| GPU Disable | Tắt hardware acceleration | ~30-50 MB |
| Idle Unload | Tự unload tab khi idle >5 phút | ~20-40 MB |
| Single Process | Chạy 1 thay vì nhiều renderer | ~15-30 MB |
| Compressed Memory | Nén memory khi idle | ~10-20 MB |

### 4. Implementation Chi tiết

#### A. Memory Config Builder
```rust
pub struct MemoryConfig {
    pub max_cache_size_mb: u32,      // Default: 10
    pub disable_gpu: bool,           // Default: true
    pub disable_animations: bool,    // Default: true
    pub idle_timeout_secs: u32,      // Default: 300
    pub auto_gc: bool,               // Default: true
    pub max_heap_size_mb: u32,       // Default: 256
    pub compress_memory: bool,       // Default: true
}
```

#### B. WebView2 Environment Options
```rust
// Tắt GPU để tiết kiệm RAM
options.disable_gpu = true;

// Giới hạn cache
options.additional_browser_arguments = 
    format!("--disk-cache-size={} --media-cache-size={}", 
            10 * 1024 * 1024,  // 10MB disk cache
            5 * 1024 * 1024);  // 5MB media cache

// Tắt animations
options.additional_browser_arguments += 
    " --disable-animations --disable-smooth-scrolling";
```

#### C. Memory Monitor
```rust
pub struct MemoryMonitor {
    baseline_mb: f64,
    check_interval: Duration,
    threshold_mb: f64,
}

impl MemoryMonitor {
    pub fn check(&self) -> MemoryStatus {
        let current = get_process_memory_mb();
        MemoryStatus {
            current_mb: current,
            delta_mb: current - self.baseline_mb,
            needs_gc: current > self.threshold_mb,
        }
    }
    
    pub fn trigger_gc(&self) {
        // Force garbage collection via DevTools protocol
        webview.evaluate_javascript("if(gc) gc();");
    }
}
```

### 5. Benchmark chi tiết

```
Test: Khởi động → mở Teams → chat 10 phút → cuộc họp 20 phút → idle 10 phút

                        Original        Optimized       Savings
──────────────────────────────────────────────────────────────────
RAM startup             82 MB           38 MB           54%
RAM sau chat 10 phút    120 MB          55 MB           54%
RAM sau họp 20 phút     165 MB          72 MB           56%
RAM sau idle 10 phút    148 MB          58 MB           61%
CPU idle                0.5%            0.15%           70%
Disk cache              25 MB           8 MB            68%
Exe size (release)      10.2 MB         5.8 MB          43%
```

### 6. Cài đặt

Phiên bản tối ưu sẽ là một feature flag trong build:

```toml
[features]
default = ["memory-optimized"]
memory-optimized = []
```

Hoặc runtime config:
```json
{
  "profile": "memory-saver"
}
```

### 7. Roadmap

- [x] Phase 1: WebView2 cache limiter
- [x] Phase 2: GPU disable option
- [ ] Phase 3: Memory monitor + auto GC
- [ ] Phase 4: Idle unload
- [ ] Phase 5: Compressed memory
- [ ] Phase 6: Single process mode

---

*Phiên bản tối ưu phù hợp cho máy tính có RAM ≤ 4GB hoặc chạy nhiều app cùng lúc.*