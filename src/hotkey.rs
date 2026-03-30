use crate::config::Config;
use evdev::KeyCode;
use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;

pub enum HotkeyEvent {
    Pressed,
    Released,
}

/// Map config key name to evdev KeyCode
fn key_from_name(name: &str) -> Option<KeyCode> {
    match name.to_lowercase().as_str() {
        "space" => Some(KeyCode::KEY_SPACE),
        "a" => Some(KeyCode::KEY_A),
        "b" => Some(KeyCode::KEY_B),
        "c" => Some(KeyCode::KEY_C),
        "d" => Some(KeyCode::KEY_D),
        "e" => Some(KeyCode::KEY_E),
        "f" => Some(KeyCode::KEY_F),
        "g" => Some(KeyCode::KEY_G),
        "h" => Some(KeyCode::KEY_H),
        "i" => Some(KeyCode::KEY_I),
        "j" => Some(KeyCode::KEY_J),
        "k" => Some(KeyCode::KEY_K),
        "l" => Some(KeyCode::KEY_L),
        "m" => Some(KeyCode::KEY_M),
        "n" => Some(KeyCode::KEY_N),
        "o" => Some(KeyCode::KEY_O),
        "p" => Some(KeyCode::KEY_P),
        "q" => Some(KeyCode::KEY_Q),
        "r" => Some(KeyCode::KEY_R),
        "s" => Some(KeyCode::KEY_S),
        "t" => Some(KeyCode::KEY_T),
        "u" => Some(KeyCode::KEY_U),
        "v" => Some(KeyCode::KEY_V),
        "w" => Some(KeyCode::KEY_W),
        "x" => Some(KeyCode::KEY_X),
        "y" => Some(KeyCode::KEY_Y),
        "z" => Some(KeyCode::KEY_Z),
        "f1" => Some(KeyCode::KEY_F1),
        "f2" => Some(KeyCode::KEY_F2),
        "f3" => Some(KeyCode::KEY_F3),
        "f4" => Some(KeyCode::KEY_F4),
        "f5" => Some(KeyCode::KEY_F5),
        "f6" => Some(KeyCode::KEY_F6),
        "f7" => Some(KeyCode::KEY_F7),
        "f8" => Some(KeyCode::KEY_F8),
        "f9" => Some(KeyCode::KEY_F9),
        "f10" => Some(KeyCode::KEY_F10),
        "f11" => Some(KeyCode::KEY_F11),
        "f12" => Some(KeyCode::KEY_F12),
        _ => None,
    }
}

/// Map modifier name to evdev KeyCode variants (any one satisfies the modifier)
fn modifier_keycodes(name: &str) -> Vec<KeyCode> {
    match name.to_lowercase().as_str() {
        "super" | "meta" | "super_l" => vec![KeyCode::KEY_LEFTMETA, KeyCode::KEY_RIGHTMETA],
        "ctrl" | "control" => vec![KeyCode::KEY_LEFTCTRL, KeyCode::KEY_RIGHTCTRL],
        "alt" => vec![KeyCode::KEY_LEFTALT, KeyCode::KEY_RIGHTALT],
        "shift" => vec![KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_RIGHTSHIFT],
        _ => vec![],
    }
}

/// Start listening for the configured hotkey combination.
/// Requires access to /dev/input/event* (user must be in `input` group).
pub fn start_listener(
    config: &Config,
) -> Result<mpsc::Receiver<HotkeyEvent>, Box<dyn std::error::Error>> {
    let trigger_key = key_from_name(&config.hotkey.key)
        .ok_or_else(|| format!("Unknown key: {}", config.hotkey.key))?;

    // Build modifier groups: each group is a set of alternative keycodes (any one satisfies)
    // e.g., "super" → [KEY_LEFTMETA, KEY_RIGHTMETA], "ctrl" → [KEY_LEFTCTRL, KEY_RIGHTCTRL]
    let mut modifier_groups: Vec<Vec<KeyCode>> = Vec::new();
    let mut all_modifier_codes: HashSet<KeyCode> = HashSet::new();

    for mod_name in &config.hotkey.modifiers {
        let codes = modifier_keycodes(mod_name);
        if codes.is_empty() {
            return Err(format!("Unknown modifier: {mod_name}").into());
        }
        for &kc in &codes {
            all_modifier_codes.insert(kc);
        }
        modifier_groups.push(codes);
    }

    if modifier_groups.is_empty() {
        return Err("No valid modifiers configured".into());
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        if let Err(e) = listen_loop(trigger_key, &modifier_groups, all_modifier_codes, tx) {
            log::error!("[hotkey] Listener error: {e}");
        }
    });

    Ok(rx)
}

fn listen_loop(
    trigger_key: KeyCode,
    modifier_groups: &[Vec<KeyCode>],
    all_modifier_codes: HashSet<KeyCode>,
    tx: mpsc::Sender<HotkeyEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let devices = find_keyboard_devices()?;
    if devices.is_empty() {
        return Err("No keyboard input devices found. Ensure user is in 'input' group.".into());
    }

    log::info!(
        "[hotkey] Monitoring {} keyboard device(s) for hotkey",
        devices.len()
    );

    let mut device = evdev::Device::open(&devices[0])?;
    log::info!("[hotkey] Listening on: {}", devices[0].display());
    log::info!(
        "[hotkey] Trigger: {} modifier group(s) + {:?}",
        modifier_groups.len(),
        trigger_key
    );
    for (i, group) in modifier_groups.iter().enumerate() {
        log::debug!("[hotkey]   Mod group {}: {:?}", i, group);
    }

    let mut keys_held: HashSet<KeyCode> = HashSet::new();
    let mut trigger_held = false;

    loop {
        match device.fetch_events() {
            Ok(events) => {
                for event in events {
                    if event.event_type() != evdev::EventType::KEY {
                        continue;
                    }
                    let code = KeyCode::new(event.code());
                    let pressed = event.value() == 1;
                    let released = event.value() == 0;

                    // Log key events at debug level
                    if pressed || released {
                        log::debug!(
                            "[hotkey] Key {:?} {}",
                            code,
                            if pressed { "↓" } else { "↑" }
                        );
                    }

                    // Track all held keys
                    if pressed {
                        keys_held.insert(code);
                    } else if released {
                        keys_held.remove(&code);
                    }

                    // Check if all modifier groups are satisfied
                    // Each group needs at least ONE of its keycodes held
                    let all_mods_satisfied = modifier_groups
                        .iter()
                        .all(|group| group.iter().any(|kc| keys_held.contains(kc)));

                    // Track trigger key
                    if code == trigger_key {
                        if pressed && !trigger_held {
                            log::debug!(
                                "[hotkey] Trigger pressed, mods_satisfied={all_mods_satisfied}"
                            );
                            if all_mods_satisfied {
                                trigger_held = true;
                                log::info!("[hotkey] ★ ACTIVATED");
                                let _ = tx.send(HotkeyEvent::Pressed);
                            }
                        } else if released && trigger_held {
                            trigger_held = false;
                            log::info!("[hotkey] ★ RELEASED");
                            let _ = tx.send(HotkeyEvent::Released);
                        }
                    }

                    // If a modifier is released while trigger is held, also release
                    if released
                        && trigger_held
                        && all_modifier_codes.contains(&code)
                        && !all_mods_satisfied
                    {
                        trigger_held = false;
                        log::info!("[hotkey] ★ RELEASED (modifier dropped)");
                        let _ = tx.send(HotkeyEvent::Released);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

fn find_keyboard_devices() -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut keyboards = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("event") {
            continue;
        }
        if let Ok(dev) = evdev::Device::open(&path) {
            if dev.supported_events().contains(evdev::EventType::KEY) {
                if let Some(keys) = dev.supported_keys() {
                    if keys.contains(KeyCode::KEY_A) && keys.contains(KeyCode::KEY_SPACE) {
                        keyboards.push(path);
                    }
                }
            }
        }
    }
    Ok(keyboards)
}
