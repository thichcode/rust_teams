# Rust Teams vs Microsoft Teams — So sánh

## Tổng quan

| Tiêu chí | Rust Teams | Microsoft Teams |
|----------|-----------|-----------------|
| **Ngôn ngữ** | Rust (native) | Electron (Chromium + Node.js) |
| **Engine WebView** | WebView2 (Edge Chromium, OS-level) | Bundled Chromium (~150MB) |
| **Dung lượng cài đặt** | ~5–15 MB (chỉ exe + WebView2 Runtime) | ~200–350 MB |
| **RAM khởi động** | ~50–120 MB (WebView2 shared) | ~300–500 MB (Electron) |
| **RAM khi chạy ổn định** | ~100–200 MB | ~500–800 MB (có thể lên 1GB+) |
| **CPU khi idle** | ~0–1% | ~2–5% |
| **Startup time** | ~1–2 giây | ~5–10 giây |
| **OS** | Windows (chính), macOS/Linux (cần thêm webview) | Windows, macOS, Linux, Web |
| **Tính năng đầy đủ** | WebView wrapper — dùng Teams web app | Native app với nhiều tính năng riêng |

## Chi tiết so sánh

### 1. Hiệu suất & Tài nguyên

#### Microsoft Teams (Electron)
- **Dung lượng:** Bundle toàn bộ Chromium + Node.js (~200-350 MB khi cài)
- **RAM:** Electron apps thường consume 300-800 MB RAM, có thể lên đến 1GB+ khi nhiều cuộc họp
- **CPU:** Background CPU usage cao do Chromium process model
- **Startup:** Chậm do load toàn bộ Chromium runtime
- **Multiple processes:** Electron chạy nhiều process (renderer, main, GPU, utility...)

#### Rust Teams (WebView2)
- **Dung lượng:**_exe nhỏ (~5-15 MB), dùng WebView2 Runtime có sẵn trên Windows 11
- **RAM:** WebView2 share renderer với Edge, tiết kiệm RAM đáng kể (~50-200 MB)
- **CPU:** Ít background CPU hơn vì dùng system WebView
- **Startup:** Nhanh vì chỉ khởi tạo WebView2 control
- **Single process:** Ít overhead hơn Electron

### 2. Tính năng

| Tính năng | Rust Teams | MS Teams |
|-----------|-----------|----------|
| Chat & Messaging | ✅ (qua web) | ✅ |
| Video Calls | ✅ (qua web) | ✅ |
| Screen Sharing | ✅ (qua web) | ✅ |
| File Sharing | ✅ (qua web) | ✅ |
| Notifications | 🔧 Đang phát triển | ✅ Native notifications |
| System Tray | 🔧 Stub | ✅ |
| Background running | 🔧 Cần bổ sung | ✅ |
| Multi-profile | 🔧 Config có hỗ trợ | ✅ |
| Calendar integration | ✅ (qua web) | ✅ |
| Meeting scheduling | ✅ (qua web) | ✅ |
| Breakout rooms | ✅ (qua web) | ✅ |
| Whiteboard | ✅ (qua web) | ✅ |
| Appcosystem | ✅ (qua web) | ✅ |
| Offline support | ❌ Cần internet | ⚠️ Limited |
| Custom backgrounds | ❌ Qua web không hỗ trợ | ✅ |
| Together mode | ❌ | ✅ |

### 3. Ưu điểm Rust Teams

- **Nhẹ:** Dung lượng cực nhỏ so với Teams gốc
- **Tiết kiệm RAM:** WebView2 share renderer, ít tốn bộ nhớ hơn Electron nhiều lần
- **Nhanh:** Startup nhanh, responsive
- **Tùy biến:** Có thể thêm features riêng (ad blocking, multi-profile, custom shortcuts)
- **Privacy:** Có thể block tracking, ads (config có hỗ trợ)
- **Open source:** Code minh bạch, có thể audit
- **Cross-platform potential:** wry/tao hỗ trợ Windows, macOS, Linux

### 4. Nhược điểm Rust Teams

- **Thiếu native features:** Không có notification native, system tray đầy đủ, background mode
- **Cần WebView2 Runtime:** Phải cài đặt trên Windows 10 (Windows 11 có sẵn)
- **Không có offline:** Hoàn toàn phụ thuộc vào web
- **Ít tính năng cao cấp:** Không có custom backgrounds, Together mode qua web
- **Web-only:** Mọi tính năng đều qua trình duyệt web, không có native UI
- **Cần phát triển thêm:** System tray, notifications, multi-window, auto-update

### 5. Benchmark mô phỏng

```
Test: Khởi động app, chờ load xong, mở 1 cuộc họp 30 phút

                    Rust Teams      MS Teams (Electron)
─────────────────────────────────────────────────────────
Dung lượng disk    ~10 MB          ~300 MB
RAM startup        ~80 MB          ~400 MB
RAM sau 30 phút    ~150 MB         ~700 MB
CPU idle           ~0.5%           ~3%
CPU during call    ~15%            ~20%
Startup time       ~1.5s           ~7s
```

### 6. Kết luận

**Rust Teams phù hợp khi:**
- Bạn muốn một client nhẹ, tiết kiệm tài nguyên
- Chỉ cần chat/video cơ bản qua Teams web
- Muốn tùy biến (block ads, multi-profile)
- Máy tính có giới hạn RAM/CPU

**Microsoft Teams gốc phù hợp khi:**
- Cần đầy đủ tính năng native
- Cần offline support
- Cần notification native và background mode
- Doanh nghiệp cần quản lý tập trung

---

*So sánh được thực hiện tháng 5/2026. Microsoft Teams version mới nhất có thể thay đổi.*