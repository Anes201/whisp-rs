use crate::config::{AudioConfig, Config, HotkeyConfig, SttConfig};
use dialoguer::{Confirm, Input, Select};
use std::process::Command;

/// Check runtime dependencies and attempt to install missing ones.
/// Returns a list of still-missing deps after install attempts.
pub fn check_and_install_deps() -> Vec<&'static str> {
    let deps: &[(&str, &str, &[&str])] = &[
        (
            "arecord",
            "alsa-utils",
            &["sudo", "apt", "install", "-y", "alsa-utils"],
        ),
        (
            "ydotool",
            "ydotool",
            &["sudo", "apt", "install", "-y", "ydotool"],
        ),
    ];

    let mut missing = Vec::new();

    for (bin, pkg, install_cmd) in deps {
        if which(bin) {
            log::info!("[deps] {bin} found");
            continue;
        }

        log::warn!("[deps] {bin} not found (package: {pkg})");

        // Attempt auto-install
        log::info!("[deps] Attempting: {}", install_cmd.join(" "));
        match Command::new(install_cmd[0])
            .args(&install_cmd[1..])
            .status()
        {
            Ok(status) if status.success() => {
                log::info!("[deps] {pkg} installed successfully");
                // Verify it's actually available now
                if !which(bin) {
                    log::error!("[deps] {pkg} installed but {bin} still not in PATH");
                    missing.push(*pkg);
                }
            }
            Ok(status) => {
                log::warn!("[deps] Install failed (exit code: {:?})", status.code());
                missing.push(*pkg);
            }
            Err(e) => {
                log::warn!("[deps] Could not run install command: {e}");
                missing.push(*pkg);
            }
        }
    }

    // Check input group membership for evdev hotkeys
    if !check_input_group() {
        log::warn!("[deps] User not in 'input' group — hotkey listener may fail");
        log::warn!("[deps] Run: sudo usermod -aG input $USER && reboot");
    }

    // Check ydotoold daemon (required for ydotool text injection)
    if which("ydotool") && !check_ydotoold() {
        log::warn!("[deps] ydotool found but ydotoold daemon not running");
        log::info!("[deps] Attempting to start ydotoold...");
        match Command::new("ydotoold")
            .arg("--socket-path=/tmp/.ydotool_socket")
            .spawn()
        {
            Ok(_) => {
                log::info!("[deps] ydotoold started");
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => {
                log::warn!("[deps] Failed to start ydotoold: {e}");
                log::warn!("[deps] Run manually: ydotoold &");
            }
        }
    }

    missing
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_input_group() -> bool {
    Command::new("groups")
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.split_whitespace().any(|g| g == "input")
        })
        .unwrap_or(false)
}

fn check_ydotoold() -> bool {
    Command::new("pgrep")
        .arg("ydotoold")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// First-run setup wizard. Returns true if config was created.
pub fn run_setup_wizard() -> Result<Option<Config>, Box<dyn std::error::Error>> {
    println!();
    println!("  ╦ ╦╦ ╦╔═╗╦╔═╔═╗╦═╗  ╦═╗╔═╗╔═╗╔═╗");
    println!("  ║║║╠═╣╠═╣╠╩╗║╣ ╠╦╝  ╠╦╝║╣ ╚═╗║ ║");
    println!("  ╚╩╝╩ ╩╩ ╩╩ ╩╚═╝╩╚═  ╩╚═╚═╝╚═╝╚═╝");
    println!();
    println!("  First-time setup. Press Enter to accept defaults.");
    println!();

    // Check if user wants to skip setup
    if !Confirm::new()
        .with_prompt("Run interactive setup?")
        .default(true)
        .interact()?
    {
        return Ok(None);
    }

    // --- API Key ---
    println!();
    println!("── Deepgram API Key ──────────────────────");

    let api_key: String = Input::new()
        .with_prompt("Deepgram API key")
        .allow_empty(true)
        .interact_text()?;

    let api_key = if api_key.is_empty() {
        // Try env var
        std::env::var("DEEPGRAM_API_KEY").ok()
    } else {
        Some(api_key)
    };

    if api_key.is_none() {
        println!();
        println!("  ⚠ No API key configured. You can add it later in:");
        println!("    {}", Config::config_path().display());
        println!("  Or set DEEPGRAM_API_KEY env var.");
        println!();
    }

    // --- Model ---
    println!();
    println!("── STT Model ─────────────────────────────");

    let models = [
        "nova-2-general (recommended, fastest)",
        "nova-2 (best accuracy)",
        "base (fastest, lower accuracy)",
    ];

    let model_idx = Select::new()
        .with_prompt("Deepgram model")
        .items(&models)
        .default(0)
        .interact()?;
    let model = models[model_idx]
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();

    // --- Language ---
    println!();
    let languages = [
        ("en", "English"),
        ("auto", "Auto-detect"),
        ("fr", "French"),
        ("es", "Spanish"),
        ("de", "German"),
        ("ar", "Arabic"),
        ("zh", "Chinese"),
        ("ja", "Japanese"),
        ("ko", "Korean"),
    ];
    let lang_labels: Vec<&str> = languages.iter().map(|(_, label)| *label).collect();
    let lang_idx = Select::new()
        .with_prompt("Dictation language")
        .items(&lang_labels)
        .default(0)
        .interact()?;
    let language = languages[lang_idx].0.to_string();

    // --- Build config ---
    let config = Config {
        hotkey: HotkeyConfig {
            modifiers: vec!["super".into()],
            key: "space".into(),
        },
        stt: SttConfig {
            api_key,
            model,
            language,
        },
        audio: AudioConfig {
            sample_rate: 16000,
            channels: 1,
        },
    };

    // Save config
    let config_path = Config::config_path();
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(&config)?;
    std::fs::write(&config_path, &toml_str)?;

    println!();
    println!("  ✓ Config saved to {}", config_path.display());
    println!();

    Ok(Some(config))
}
