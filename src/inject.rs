use std::process::Command;

/// Inject text at the current cursor position.
/// Tries: wtype → clipboard+paste → ydotool
pub fn inject_text(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    if text.is_empty() {
        return Ok(());
    }

    log::info!(
        "[inject] Injecting {} chars: \"{}\"",
        text.len(),
        truncate(text, 80)
    );

    // Strategy 1: wtype (native Wayland typer, no daemon needed)
    if which("wtype") {
        log::debug!("[inject] Trying wtype...");
        match run_wtype(text) {
            Ok(()) => {
                log::info!("[inject] ✓ wtype succeeded");
                return Ok(());
            }
            Err(e) => {
                log::warn!("[inject] wtype failed: {e}");
            }
        }
    }

    // Strategy 2: clipboard + Ctrl+V paste (works everywhere)
    if which("wl-copy") {
        log::debug!("[inject] Trying clipboard paste...");
        match clipboard_paste(text) {
            Ok(()) => {
                log::info!("[inject] ✓ clipboard paste succeeded");
                return Ok(());
            }
            Err(e) => {
                log::warn!("[inject] clipboard paste failed: {e}");
            }
        }
    }

    // Strategy 3: ydotool (needs ydotoold daemon)
    if which("ydotool") {
        if check_ydotoold() {
            log::debug!("[inject] Trying ydotool...");
            match run_ydotool(text) {
                Ok(()) => {
                    log::info!("[inject] ✓ ydotool succeeded");
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("[inject] ydotool failed: {e}");
                }
            }
        } else {
            log::warn!("[inject] ydotool found but ydotoold daemon not running");
        }
    }

    Err("All injection methods failed. Install wtype or wl-clipboard.".into())
}

fn run_ydotool(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("ydotool")
        .args(["type", "--key-delay", "10", text])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ydotool exit {}: {}", output.status, stderr.trim()).into());
    }
    Ok(())
}

fn run_wtype(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("wtype").args(["-d", "10", text]).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wtype exit {}: {}", output.status, stderr.trim()).into());
    }
    Ok(())
}

fn clipboard_paste(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Copy to clipboard via wl-copy
    if !which("wl-copy") {
        return Err("wl-copy not found".into());
    }

    let mut child = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        stdin.write_all(text.as_bytes())?;
        drop(stdin); // close stdin to signal EOF
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("wl-copy failed: {status}").into());
    }

    // Small delay to let clipboard settle
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Paste via Ctrl+V using ydotool key simulation
    if which("ydotool") && check_ydotoold() {
        // ydotool key: Ctrl+V (key code 29=LeftCtrl, 47=V)
        let _ = Command::new("ydotool")
            .args(["key", "29:1", "47:1", "47:0", "29:0"])
            .status()?;
        return Ok(());
    }

    // Or via wtype
    if which("wtype") {
        let _ = Command::new("wtype").args(["-M", "ctrl", "v"]).status()?;
        return Ok(());
    }

    Err("No key simulator available for paste".into())
}

fn check_ydotoold() -> bool {
    // Check if ydotoold process is running
    Command::new("pgrep")
        .arg("ydotoold")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
