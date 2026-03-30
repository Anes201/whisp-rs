mod api;
mod applet;
mod audio;
mod config;
mod hotkey;
mod inject;
mod setup;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle CLI flags before anything else
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--set-api-key" {
        return set_api_key(&args[2]);
    }
    if args.len() >= 2 && args[1] == "--setup" {
        return run_setup();
    }

    // Structured logging with timestamps, module paths, and levels
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("whisp_rs=info"))
        .format_timestamp_secs()
        .format_module_path(false)
        .format_target(false)
        .init();

    log::info!("=== whisp-rs starting ===");

    // 1. Check and install runtime dependencies
    let missing_deps = setup::check_and_install_deps();
    if !missing_deps.is_empty() {
        log::error!("Missing dependencies: {}", missing_deps.join(", "));
        eprintln!();
        eprintln!("  ✗ Missing deps. Install manually:");
        eprintln!("    sudo apt install -y {}", missing_deps.join(" "));
        eprintln!();
        // Continue anyway — some features will degrade gracefully
    }

    // 2. Load or create config (first-run wizard if no config exists)
    let config = if !config::Config::config_path().exists() {
        log::info!("No config found, launching setup wizard");
        match setup::run_setup_wizard()? {
            Some(cfg) => cfg,
            None => {
                log::info!("Setup skipped, using defaults");
                config::Config::default()
            }
        }
    } else {
        config::Config::load()?
    };

    log::info!(
        "Config loaded: hotkey={:?}+{}, model={}, language={}",
        config.hotkey.modifiers,
        config.hotkey.key,
        config.stt.model,
        config.stt.language,
    );
    log::info!(
        "API key: {}",
        if config.stt.api_key.is_some() {
            "set"
        } else {
            "missing"
        },
    );

    // 3. Start system tray
    applet::set_hotkey_label(&config.hotkey.modifiers, &config.hotkey.key);
    let _tray_handle = applet::start_tray()?;
    log::info!("[tray] System tray started (idle state)");

    // 4. Start hotkey listener
    let hotkey_rx = hotkey::start_listener(&config)?;
    log::info!(
        "[hotkey] Listening for {:?}+{} on /dev/input",
        config.hotkey.modifiers,
        config.hotkey.key,
    );

    // 5. Initialize STT client
    let deepgram_client = config.stt.api_key.as_ref().map(|key| {
        log::info!(
            "[stt] Deepgram client initialized (model={}, key={}...)",
            config.stt.model,
            &key[..8.min(key.len())],
        );
        api::deepgram::DeepgramClient::new(
            key.clone(),
            config.stt.model.clone(),
            config.stt.language.clone(),
        )
    });

    if deepgram_client.is_none() {
        log::error!("[stt] No API key configured! Set DEEPGRAM_API_KEY or run: cargo run -- --set-api-key <key>");
    }

    // 6. Main event loop
    let audio_capture = audio::AudioCapture::new(config.audio.sample_rate, config.audio.channels);
    let mut audio_handle: Option<audio::AudioHandle> = None;
    let mut dictation_count: u64 = 0;

    log::info!("=== whisp-rs ready. Hold Super+Space to dictate ===");

    loop {
        match hotkey_rx.recv() {
            Ok(hotkey::HotkeyEvent::Pressed) => {
                log::info!("[cycle] Hotkey PRESSED → starting audio capture");
                applet::set_state(applet::STATE_RECORDING);

                match audio_capture.start() {
                    Ok(handle) => {
                        audio_handle = Some(handle);
                        log::debug!("[audio] arecord subprocess spawned");
                    }
                    Err(e) => {
                        log::error!("[audio] Failed to start capture: {e}");
                        log::error!("[audio] Is arecord installed? Is a microphone connected?");
                        applet::set_state(applet::STATE_IDLE);
                    }
                }
            }
            Ok(hotkey::HotkeyEvent::Released) => {
                log::info!("[cycle] Hotkey RELEASED → stopping capture, processing");
                applet::set_state(applet::STATE_PROCESSING);

                if let Some(handle) = audio_handle.take() {
                    let pcm = handle.drain_pcm();
                    let duration_ms = (pcm.len() as f64 / (config.audio.sample_rate as f64 * 2.0)
                        * 1000.0) as u64;
                    log::info!(
                        "[audio] Captured {} bytes (~{}ms of audio)",
                        pcm.len(),
                        duration_ms
                    );

                    if pcm.is_empty() {
                        log::warn!("[audio] Empty capture — was the mic muted?");
                        applet::set_state(applet::STATE_IDLE);
                        continue;
                    }

                    // Transcribe
                    let t_start = std::time::Instant::now();
                    let text = match &deepgram_client {
                        Some(client) => {
                            log::debug!("[stt] Sending to Deepgram ({}ms audio)...", duration_ms);
                            client.transcribe(&pcm)
                        }
                        None => Err("No Deepgram API key configured".into()),
                    };
                    let t_elapsed = t_start.elapsed();

                    match text {
                        Ok(t) if !t.is_empty() => {
                            dictation_count += 1;
                            log::info!(
                                "[stt] Transcribed in {}ms (dictation #{}): \"{}\"",
                                t_elapsed.as_millis(),
                                dictation_count,
                                t
                            );
                            if let Err(e) = inject::inject_text(&t) {
                                log::error!("[inject] Text injection failed: {e}");
                                log::error!(
                                    "[inject] Is ydotool installed and running (ydotoold)?"
                                );
                            }
                        }
                        Ok(_) => {
                            log::warn!(
                                "[stt] Empty transcription after {}ms — no speech detected?",
                                t_elapsed.as_millis()
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "[stt] Transcription failed after {}ms: {e}",
                                t_elapsed.as_millis()
                            );
                        }
                    }
                } else {
                    log::warn!("[cycle] Released but no audio handle — capture may have failed");
                }

                applet::set_state(applet::STATE_IDLE);
            }
            Err(e) => {
                log::error!("[hotkey] Channel error: {e} — listener thread died?");
                break;
            }
        }
    }

    log::info!(
        "=== whisp-rs shutting down ({} dictations) ===",
        dictation_count
    );
    Ok(())
}

fn set_api_key(key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config::Config::config_path();

    // Load existing config or create default
    let mut cfg: toml::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        toml::from_str(&content)?
    } else {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        toml::from_str(include_str!("../default_config.toml"))?
    };

    // Set the key
    cfg["stt"]["api_key"] = toml::Value::String(key.to_string());

    // Write back
    std::fs::write(&config_path, toml::to_string_pretty(&cfg)?)?;
    println!("  ✓ API key saved to {}", config_path.display());
    Ok(())
}

fn run_setup() -> Result<(), Box<dyn std::error::Error>> {
    match setup::run_setup_wizard()? {
        Some(_) => println!("  ✓ Setup complete. Run `cargo run` to start."),
        None => println!("  Setup cancelled."),
    }
    Ok(())
}
