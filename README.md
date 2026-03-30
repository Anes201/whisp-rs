# whisp-rs

Lightweight voice-to-text dictation for Linux (Wayland/COSMIC). Hold a hotkey, speak, release — text appears at your cursor.

Single binary. No local models. No bloat.

[![Made with Rust](https://img.shields.io/badge/Made%20with-Rust-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## How it works

1. **Hold** `Ctrl+Space`
2. **Speak** into your microphone
3. **Release** — your words are typed at the cursor

Audio is captured via ALSA, sent to [Deepgram](https://deepgram.com/) for transcription, and injected into the focused window using `wtype` or clipboard paste.

## Features

- **Global hotkey** via `evdev` — works in any app, any workspace
- **System tray** icon shows state: idle / recording / processing
- **Multiple injection methods**: `wtype` → clipboard+paste → `ydotool` (auto-fallback)
- **First-run setup wizard** — no manual config editing needed
- **Raw PCM streaming** — no WAV overhead, minimal latency
- **Configurable**: model, language, hotkey, sample rate
- **Tiny binary** (~2MB release build with LTO)

## Requirements

| Dependency | Purpose | Install |
|---|---|---|
| `arecord` | Audio capture (ALSA) | `sudo apt install alsa-utils` |
| `wtype` or `wl-clipboard` | Text injection | `sudo apt install wtype` or `sudo apt install wl-clipboard` |
| `ydotool` | Fallback injection | `sudo apt install ydotool` |
| `input` group | Hotkey access | `sudo usermod -aG input $USER && reboot` |

**OS**: Linux with Wayland (tested on Pop!_OS COSMIC, should work on GNOME, KDE, Sway, Hyprland)

## Install

### One-shot installer (Debian/Ubuntu/Pop!_OS)

No Rust required. Downloads the pre-built binary and installs all system deps:

```bash
curl -sSL https://raw.githubusercontent.com/Anes201/whisp-rs/main/install.sh | bash
```

### Pre-built binary

Download from [GitHub Releases](https://github.com/Anes201/whisp-rs/releases), extract, and run:

```bash
tar xzf whisp-rs-x86_64-linux.tar.gz
sudo mv whisp-rs /usr/local/bin/
whisp-rs --setup
```

### From crates.io (requires Rust)

```bash
cargo install whisp-rs
```

### From source

```bash
git clone https://github.com/anes201/whisp-rs.git
cd whisp-rs
cargo build --release
# Binary at: target/release/whisp-rs
```

## Setup

### First run (automatic wizard)

```bash
whisp-rs
```

If no config exists, the setup wizard launches automatically. It asks for:
- **Hotkey** (default: `Ctrl+Space`)
- Deepgram API key ([get one free](https://console.deepgram.com/signup) — $200 credit)
- STT model (default: `nova-2-general`)
- Language

### Manual setup

```bash
# Set API key only
whisp-rs --set-api-key YOUR_DEEPGRAM_API_KEY

# Re-run full wizard
whisp-rs --setup
```

### Environment variable

```bash
export DEEPGRAM_API_KEY="your-key-here"
whisp-rs
```

## Configuration

Config file: `~/.config/whisp-rs/config.toml`

```toml
[hotkey]
modifiers = ["ctrl"]        # ctrl | super | alt | shift (or combo: ["ctrl", "shift"])
key = "space"

[stt]
api_key = "your-deepgram-api-key"
model = "nova-2-general"    # nova-2-general | nova-2 | base
language = "en"             # en, fr, es, de, ar, zh, ja, ko, auto

[audio]
sample_rate = 16000
channels = 1
```

### Models

| Model | Speed | Accuracy | Best for |
|---|---|---|---|
| `nova-2-general` | Fastest | Good | Dictation (default) |
| `nova-2` | Fast | Best | Accuracy-critical |
| `base` | Fastest | Lower | Quick notes |

## CLI flags

```
whisp-rs                    # Start dictation daemon
whisp-rs --set-api-key KEY  # Save API key to config
whisp-rs --setup            # Re-run setup wizard
whisp-rs --help             # Show help
```

## Logging

```bash
# Default: info level
whisp-rs

# Debug: see API calls, hotkey events, injection attempts
RUST_LOG=whisp_rs=debug whisp-rs

# Trace: everything
RUST_LOG=whisp_rs=trace whisp-rs
```

## Troubleshooting

**"No keyboard input devices found"**
```bash
sudo usermod -aG input $USER
# Log out and back in (or reboot)
```

**"All injection methods failed"**
```bash
# Install wtype (recommended for Wayland)
sudo apt install wtype

# Or install wl-clipboard + ydotool
sudo apt install wl-clipboard ydotool
ydotoold &
```

**"Missing dependencies: arecord"**
```bash
sudo apt install alsa-utils
```

**Empty transcriptions**
- Check your microphone is connected and not muted
- Run with `RUST_LOG=whisp_rs=debug` to see audio capture sizes
- Test mic: `arecord -f S16_LE -r 16000 -c 1 -t raw -d 5 /tmp/test.raw && aplay -f S16_LE -r 16000 -c 1 /tmp/test.raw`

## Architecture

```
┌─────────────┐     ┌──────────┐     ┌─────────────┐
│  evdev       │────▶│  arecord │────▶│  Deepgram   │
│  hotkey      │     │  (ALSA)  │     │  API        │
└─────────────┘     └──────────┘     └──────┬──────┘
                                            │
┌─────────────┐     ┌──────────┐            │
│  ksni        │     │  wtype / │◀───────────┘
│  tray icon   │     │  inject  │
└─────────────┘     └──────────┘
```

- **Hotkey**: `evdev` reads raw keyboard events from `/dev/input/`
- **Audio**: `arecord` subprocess captures raw PCM (16kHz, 16-bit, mono)
- **STT**: Deepgram REST API with raw PCM streaming (no WAV overhead)
- **Injection**: `wtype` → `wl-copy`+`ydotool` → `ydotool type` (auto-fallback)
- **Tray**: `ksni` system tray with state indicators

## Support

If this project saves you time, consider buying me a coffee:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/anes201)

## License

MIT
