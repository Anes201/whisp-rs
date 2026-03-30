use std::io::Read;
use std::process::{Child, Command, Stdio};

pub struct AudioCapture {
    sample_rate: u32,
    channels: u16,
}

impl AudioCapture {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    /// Start capturing audio via arecord (ALSA, pre-installed on Pop!_OS).
    /// Returns a handle; call drain_pcm() after dropping to get recorded bytes.
    pub fn start(&self) -> Result<AudioHandle, Box<dyn std::error::Error>> {
        log::debug!(
            "[audio] Spawning arecord: {}Hz, {}ch, format=S16_LE",
            self.sample_rate,
            self.channels
        );

        let child = Command::new("arecord")
            .args([
                "-f",
                "S16_LE",
                "-r",
                &self.sample_rate.to_string(),
                "-c",
                &self.channels.to_string(),
                "-t",
                "raw",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // capture errors
            .spawn()?;

        log::info!(
            "[audio] arecord started (pid={}, {}Hz, {}ch)",
            child.id(),
            self.sample_rate,
            self.channels
        );
        Ok(AudioHandle { child })
    }
}

pub struct AudioHandle {
    child: Child,
}

impl AudioHandle {
    /// Stop recording and return all captured PCM bytes (i16 LE).
    pub fn drain_pcm(mut self) -> Vec<u8> {
        // Kill arecord FIRST so stdout pipe closes and read_to_end doesn't block
        let _ = self.child.kill();

        // Now read from stdout (pipe is closed, returns immediately)
        let mut stdout = match self.child.stdout.take() {
            Some(s) => s,
            None => {
                log::error!("[audio] arecord stdout was None");
                return Vec::new();
            }
        };

        let mut buf = Vec::new();
        match stdout.read_to_end(&mut buf) {
            Ok(n) => log::debug!("[audio] Read {n} bytes from arecord stdout"),
            Err(e) => log::error!("[audio] Failed to read arecord stdout: {e}"),
        }

        // Capture stderr for diagnostics
        if let Some(mut stderr) = self.child.stderr.take() {
            let mut err_buf = String::new();
            if stderr.read_to_string(&mut err_buf).is_ok() && !err_buf.is_empty() {
                log::debug!("[audio] arecord stderr: {}", err_buf.trim());
            }
        }

        // Reap the process
        match self.child.wait() {
            Ok(status) => {
                if !status.success() {
                    log::debug!("[audio] arecord exit: {status} (expected after kill)");
                }
            }
            Err(e) => log::error!("[audio] Failed to wait on arecord: {e}"),
        }

        log::info!(
            "[audio] Captured {} bytes of PCM (~{:.0}ms)",
            buf.len(),
            buf.len() as f64 / (16000.0 * 2.0) * 1000.0
        );
        buf
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        log::debug!("[audio] arecord process killed");
    }
}
