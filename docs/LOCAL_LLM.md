# Local LLM Mode

R Teams can run STT, translation, and suggestions entirely offline using:
- **whisper.cpp** for Speech-to-Text (auto-downloaded)
- **Ollama** for translation and suggested replies

No API keys needed. No internet required once models are installed.

## How It Works

```
Microphone → whisper.cpp → text → Ollama (translate) → translation
                                         → Ollama (suggest) → suggestions
```

## Setup

### 1. Install Ollama

Download from [ollama.com](https://ollama.com) and install.

Pull at least one translation-capable model:

```bash
ollama pull llama3.2:3b
ollama pull qwen2.5:7b
```

### 2. Open the Local Mode Wizard

1. Click **Realtime Translate** panel → **🖥 Local** button
2. Pick your STT model (whisper.cpp is auto-detected)
3. Pick your Translator model (from installed Ollama models)
4. Pick your Suggester model (from installed Ollama models)
5. Click **✓ Apply**

### 3. Verify Readiness

After applying, R Teams checks:
- **Ollama** — running at `http://localhost:11434`, has the selected models
- **Whisper** — binary and model files are downloaded

A banner shows:
- **Green** — everything ready, pipeline uses local providers
- **Amber** — partial readiness, check Configure for details

## Hybrid Providers

The Local mode wizard sets defaults, but you can mix providers:

| Role | Local Option | Cloud Option |
|------|-------------|--------------|
| STT | whisper.cpp | OpenAI Whisper |
| Translator | Ollama model | OpenAI GPT-4 |
| Suggester | Ollama model | OpenAI GPT-4 |

Use the **Configure** button after the wizard to fine-tune per-provider settings.

## Configuration

Stored in `%APPDATA%/com/rust-teams/app/config.json`:

```json
{
  "local_preset": {
    "stt_model": "...",
    "translator_model": "...",
    "suggester_model": "...",
    "ollama_endpoint": "http://localhost:11434",
    "whisper_binary": "...whisper/bin/whisper-cli.exe",
    "whisper_model": "...whisper/models/ggml-tiny.en.bin",
    "last_checked": "2026-06-04T12:00:00Z"
  },
  "stt": { "provider_type": "local", "api_url": "...", "api_key": "..." },
  "translator": { "provider_type": "ollama", ... },
  "suggester": { "provider_type": "ollama", ... }
}
```

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Banner shows amber | Ollama not running | Start Ollama, run `ollama serve` |
| Banner shows amber | Whisper files missing | Re-open wizard, or wait for auto-download |
| "No models available" in wizard | Ollama not running | Start Ollama, pull a model |
| STT fails silently | Whisper binary not found | Check `whisper_binary` in config.json |
| Translation slow | Small model on weak CPU | Pull a larger model for better quality |

## Whisper Auto-Download

On first local setup, R Teams downloads:
- **whisper-cli.exe** (~5 MB) from GitHub releases
- **ggml-tiny.en.bin** (~75 MB) from HuggingFace

Stored in `%APPDATA%/RustTeams/whisper/`. Retries 3 times with 5-minute timeout.
