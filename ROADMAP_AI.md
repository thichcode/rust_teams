# 🧠 Rust Teams — AI Local Features Roadmap

## Vision
Tích hợp AI chạy local (on-device) vào Rust Teams để tạo trải nghiệm thông minh, riêng tư, không phụ thuộc cloud AI.

---

## Phase 1: AI-powered Features (Q3 2026)

### 1.1 🔍 Smart Search & Summarize
- **Mô tả:** AI tóm tắt nội dung chat, meeting notes, documents
- **Model:** Phi-3 Mini (3.8B) hoặc Qwen2-1.5B
- **Library:** `llama-cpp-rs` (GGUF format)
- **RAM:** +200-400 MB khi inference
- **Use case:**
  ```
  User: "Tóm tắt cuộc họp hôm nay"
  AI: "Cuộc họp thảo luận 3 chủ đề: 1) Release v2.0 vào thứ 6, 
       2) Bug #123 cần fix trước, 3) Plan sprint tiếp theo..."
  ```

### 1.2 💬 Smart Reply Suggestions
- **Mô tả:** Gợi ý câu trả lời nhanh dựa trên context tin nhắn
- **Model:** Phi-3 Mini hoặc DistilGPT2 (nhẹ hơn)
- **Implementation:**
  - Local inference trên message context
  - Gợi ý 3-5 câu trả lời phù hợp
  - Hiển thị inline khi người dùng hover
- **Latency:** <500ms trên CPU, <200ms trên GPU

### 1.3 🌐 Real-time Translation
- **Mô tả:** Dịch tin nhắn real-time giữa các ngôn ngữ
- **Model:** NLLB-200 (No Language Left Behind) hoặc MarianMT
- **Library:** `candle` (Rust ML framework) hoặc `ort` (ONNX Runtime)
- **Features:**
  - Auto-detect language
  - Dịch inline trong chat
  - Hỗ trợ 100+ ngôn ngữ
  - Offline hoàn toàn

---

## Phase 2: Advanced AI (Q4 2026)

### 2.1 📝 Meeting Notes Auto-generation
- **Mô tả:** Tự động ghi chú cuộc họp từ audio
- **Pipeline:**
  ```
  Audio → Whisper.cpp (STT) → Text → Phi-3 (Summarize) → Notes
  ```
- **Models:**
  - STT: Whisper.cpp (tiny/base model)
  - Summarize: Phi-3 Mini
- **RAM:** +300-500 MB
- **Storage:** +500MB models

### 2.2 🎯 Action Item Detection
- **Mô tả:** Tự động phát hiện action items từ cuộc họp
- **Example:**
  ```
  AI detected: "John sẽ gửi report trước thứ 6"
  → Tạo task автоматически trong Teams
  ```
- **Model:** Phi-3 Mini với custom fine-tune

### 2.3 📊 Sentiment Analysis
- **Mô tả:** Phân tích cảm xúc trong tin nhắn
- **Model:** DistilBERT fine-tuned cho sentiment
- **Features:**
  - Real-time sentiment score
  - Alert khi sentiment tiêu cực
  - Dashboard sentiment team

---

## Phase 3: AI Assistant (Q1 2027)

### 3.1 🤖 Local AI Assistant
- **Mô tả:** Trợ lý AI chạy hoàn toàn local
- **Model:** Phi-3 Medium (14B) hoặc Qwen2-7B
- **Features:**
  ```
  User: "Ai đã gửi report tuần này?"
  AI: "Tuần này có 3 người gửi: John (Monday), 
       Sarah (Wednesday), Mike (Friday)..."
  
  User: "Tóm tắt kênh #engineering tuần này"
  AI: "Kênh có 47 tin nhắn. Chủ đề chính: 
       1) Bug fixes, 2) New API, 3) Performance..."
  ```
- **RAM:** +1-2 GB
- **GPU:** CUDA/Metal acceleration optional

### 3.2 📧 Smart Email Draft
- **Mô tả:** Viết draft email dựa trên context chat
- **Model:** Phi-3 Mini
- **Features:**
  - Phân tích context cuộc trò chuyện
  - Gợi ý email draft
  - Tùy chỉnh tone (formal/casual)

### 3.3 🔔 Intelligent Notifications
- **Mô tả:** AI ưu tiên thông báo quan trọng
- **Model:** Lightweight classifier
- **Features:**
  - Phân loại tin nhắn: urgent/normal/low
  - Auto-mute kênh không quan trọng
  - Smart digest (tóm tắt hàng ngày)

---

## Phase 4: Enterprise AI (Q2 2027)

### 4.1 🔐 Privacy-first AI
- **Mô tả:** AI chạy hoàn toàn local, không gửi data lên cloud
- **Certification:** SOC2, GDPR compliance
- **Features:**
  - On-device inference
  - Encrypted model storage
  - Audit trail cho AI decisions

### 4.2 🏢 Knowledge Base AI
- **Mô tả:** AI tìm kiếm và trả lời từ knowledge base nội bộ
- **RAG Pipeline:**
  ```
  Documents → Embedding (MiniLM) → Vector DB (local)
  Query → Embedding → Similarity Search → Context → LLM → Answer
  ```
- **Vector DB:** SQLite + `sqlite-vec` hoặc `tantivy`
- **Model:** Phi-3 Mini + MiniLM embeddings

### 4.3 📈 Analytics Dashboard
- **Mô tả:** AI phân tích năng suất team
- **Metrics:**
  - Response time analysis
  - Meeting frequency
  - Communication patterns
  - Team sentiment trends
- **Visualization:** Charts trong app

---

## Technical Architecture

```
┌─────────────────────────────────────────────────┐
│                  Rust Teams                      │
├─────────────────────────────────────────────────┤
│  UI Layer (tao + wry)                           │
├─────────────────────────────────────────────────┤
│  AI Engine (local)                              │
│  ┌─────────────┐  ┌─────────────┐              │
│  │ LLM Engine  │  │ STT Engine  │              │
│  │ (llama-cpp) │  │ (whisper)   │              │
│  └─────────────┘  └─────────────┘              │
│  ┌─────────────┐  ┌─────────────┐              │
│  │ Embeddings  │  │ Classifier  │              │
│  │ (MiniLM)    │  │ (BERT)      │              │
│  └─────────────┘  └─────────────┘              │
├─────────────────────────────────────────────────┤
│  Model Storage (encrypted)                      │
│  ~/.rust_teams/models/*.gguf                    │
├─────────────────────────────────────────────────┤
│  WebView2 (Teams web app)                       │
└─────────────────────────────────────────────────┘
```

## Dependencies Mới

```toml
[dependencies]
# AI/ML
llama-cpp-rs = "0.3"        # LLM inference
whisper-rs = "0.11"          # Speech-to-text
candle = "0.8"               # ML framework (alternative)
ort = "2.0"                  # ONNX Runtime

# Embeddings
fastembed = "0.14"           # Text embeddings

# Vector Search
tantivy = "0.22"             # Full-text search
sqlite-vec = "0.1"           # Vector similarity

# Audio
cpal = "0.15"                # Audio capture
```

## RAM Budget

```
Base app (WebView2):          80 MB
+ Phi-3 Mini (3.8B Q4):     250 MB
+ Whisper Tiny:               75 MB
+ Embeddings (MiniLM):        50 MB
+ Vector DB:                  30 MB
─────────────────────────────────
Total khi chạy đầy đủ:      485 MB
(vẫn thấp hơn MS Teams gốc: 700 MB)
```

## Model Distribution

```
models/
├── phi-3-mini-3.8b-q4.gguf    (~2.2 GB)
├── whisper-tiny.en.gguf        (~75 MB)
├── whisper-base.gguf           (~142 MB)
├── minilm-l6-v2.onnx           (~22 MB)
└── nllb-200-distilled.onnx     (~500 MB)

Total: ~3 GB (download lần đầu)
```

## Priority Matrix

| Feature | Impact | Effort | Priority |
|---------|--------|--------|----------|
| Smart Reply | ⭐⭐⭐⭐⭐ | Medium | 🔴 P0 |
| Translation | ⭐⭐⭐⭐ | Medium | 🔴 P0 |
| Summarize | ⭐⭐⭐⭐⭐ | High | 🟡 P1 |
| Meeting Notes | ⭐⭐⭐⭐ | High | 🟡 P1 |
| Sentiment | ⭐⭐⭐ | Low | 🟢 P2 |
| Local Assistant | ⭐⭐⭐⭐⭐ | Very High | 🟢 P2 |
| Knowledge Base | ⭐⭐⭐⭐⭐ | Very High | 🔵 P3 |

---

## Timeline

```
Q3 2026: Smart Reply + Translation (Phase 1)
Q4 2026: Meeting Notes + Action Items (Phase 2)
Q1 2027: Local Assistant (Phase 3)
Q2 2027: Enterprise AI (Phase 4)
```

---

*Rust Teams — AI-powered, privacy-first, runs on your machine.*