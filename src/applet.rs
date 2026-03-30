use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use ksni::{Category, MenuItem, ToolTip, Tray};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// Tray states: 0=Idle, 1=Recording, 2=Processing
static STATE: AtomicU8 = AtomicU8::new(0);
static HOTKEY_LABEL: OnceLock<String> = OnceLock::new();

pub const STATE_IDLE: u8 = 0;
pub const STATE_RECORDING: u8 = 1;
pub const STATE_PROCESSING: u8 = 2;

pub fn set_state(state: u8) {
    STATE.store(state, Ordering::Relaxed);
}

/// Set the hotkey label shown in the tray tooltip.
/// Call once at startup before spawning the tray.
pub fn set_hotkey_label(modifiers: &[String], key: &str) {
    let label = if modifiers.is_empty() {
        key.to_uppercase()
    } else {
        format!(
            "{}+{}",
            modifiers
                .iter()
                .map(|m| match m.as_str() {
                    "super" | "meta" => "Super".to_string(),
                    "ctrl" | "control" => "Ctrl".to_string(),
                    "alt" => "Alt".to_string(),
                    "shift" => "Shift".to_string(),
                    other => other.to_string(),
                })
                .collect::<Vec<_>>()
                .join("+"),
            key.to_uppercase()
        )
    };
    let _ = HOTKEY_LABEL.set(label);
}

pub struct WhispTray;

impl Tray for WhispTray {
    fn id(&self) -> String {
        "whisp-rs".into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn title(&self) -> String {
        "whisp-rs".into()
    }

    fn icon_name(&self) -> String {
        match STATE.load(Ordering::Relaxed) {
            1 => "microphone-sensitivity-high".into(),
            2 => "process-working".into(),
            _ => "microphone-sensitivity-muted".into(),
        }
    }

    fn icon_theme_path(&self) -> String {
        String::new()
    }

    fn tool_tip(&self) -> ToolTip {
        let state_str = match STATE.load(Ordering::Relaxed) {
            1 => "Recording...",
            2 => "Processing...",
            _ => {
                let hotkey = HOTKEY_LABEL
                    .get()
                    .map(|s| s.as_str())
                    .unwrap_or("Ctrl+Space");
                &format!("Idle - Hold {hotkey} to dictate")
            }
        };
        ToolTip {
            title: "whisp-rs".into(),
            description: state_str.to_string(),
            icon_name: self.icon_name(),
            icon_pixmap: vec![],
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![MenuItem::Standard(StandardItem {
            label: "Quit".into(),
            icon_name: "application-exit".into(),
            enabled: true,
            visible: true,
            ..Default::default()
        })]
    }
}

/// Start the system tray. Returns a handle for shutdown.
pub fn start_tray() -> Result<ksni::blocking::Handle<WhispTray>, Box<dyn std::error::Error>> {
    let handle = WhispTray.spawn()?;
    log::info!("System tray started");
    Ok(handle)
}
