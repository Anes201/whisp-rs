use reqwest::blocking::Client;
use serde_json::Value;

const DEEPGRAM_API_URL: &str = "https://api.deepgram.com/v1/listen";

pub struct DeepgramClient {
    client: Client,
    api_key: String,
    model: String,
    language: String,
}

impl DeepgramClient {
    pub fn new(api_key: String, model: String, language: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            language,
        }
    }

    /// Transcribe raw PCM audio to text (optimized for speed).
    /// Sends raw PCM bytes with encoding/sample_rate params — no WAV overhead.
    pub fn transcribe(&self, pcm_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let duration_ms = pcm_data.len() as f64 / (16000.0 * 2.0) * 1000.0;
        log::debug!(
            "[deepgram] Uploading {} bytes (~{:.0}ms raw PCM) to {}",
            pcm_data.len(),
            duration_ms,
            DEEPGRAM_API_URL
        );

        // Build URL with speed-optimized params:
        // - nova-2-general: fastest general model
        // - punctuate=false: skip punctuation for lower latency
        // - encoding=linear16&sample_rate=16000: raw PCM, no WAV headers
        let mut url = format!(
            "{}?model={}&encoding=linear16&sample_rate=16000&punctuate=false",
            DEEPGRAM_API_URL, self.model
        );
        if self.language != "auto" {
            url.push_str(&format!("&language={}", self.language));
        }

        let t_start = std::time::Instant::now();
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "application/octet-stream")
            .body(pcm_data.to_vec())
            .send()?;
        let http_ms = t_start.elapsed().as_millis();

        log::debug!("[deepgram] HTTP {} in {}ms", response.status(), http_ms);

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            log::error!("[deepgram] API error {status}: {body}");
            return Err(format!("Deepgram API error {status}: {body}").into());
        }

        let json: Value = response.json()?;
        let text = json["results"]["channels"][0]["alternatives"][0]["transcript"]
            .as_str()
            .unwrap_or("")
            .to_string();

        log::debug!("[deepgram] Response: \"{}\"", text);
        Ok(text)
    }
}
