use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use ksni::{Category, MenuItem, ToolTip, Tray};
use std::sync::atomic::{AtomicU8, Ordering};

/// Tray states: 0=Idle, 1=Recording, 2=Processing
static STATE: AtomicU8 = AtomicU8::new(0);

pub const STATE_IDLE: u8 = 0;
pub const STATE_RECORDING: u8 = 1;
pub const STATE_PROCESSING: u8 = 2;

pub fn set_state(state: u8) {
    STATE.store(state, Ordering::Relaxed);
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
            _ => "Idle - Hold Super+Space to dictate",
        };
        ToolTip {
            title: "whisp-rs".into(),
            description: state_str.into(),
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
