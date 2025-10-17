#[derive(Debug, Clone)]
pub struct Config {
    pub accept_language: String,
    pub platform: String,
    pub language: String,
    pub languages: Vec<String>,
    pub screen_width: u32,
    pub screen_height: u32,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
    pub hardware_concurrency: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            accept_language: "en-US,en;q=0.9".to_string(),
            platform: "Win32".to_string(),
            language: "en-US".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            screen_width: 1920,
            screen_height: 1080,
            webgl_vendor: "Intel Inc.".to_string(),
            webgl_renderer: "Intel(R) UHD Graphics".to_string(),
            hardware_concurrency: 8,
        }
    }
}
