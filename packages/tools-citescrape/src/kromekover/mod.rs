use anyhow::Result;
use chromiumoxide::{Page, cdp};
use futures::future::join_all;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

mod config;
use config::Config;

// Order matters! Scripts are injected in this sequence for maximum stealth
const EVASION_SCRIPTS: &[&str] = &[
    // Proxy utilities MUST be loaded first (dependency for core_utils and others)
    "evasions/proxy_utils.js", // Proxy manipulation utilities (required by core_utils.js)
    // Core utilities and helpers
    "evasions/core_utils.js", // Shared utility functions (depends on proxy_utils.js)
    // ChromeDriver detection evasion (must run early)
    "evasions/cdp_evasion.js", // Delete CDP detection variables
    // Navigator properties (most basic checks first)
    "evasions/navigator_webdriver.js",   // Remove webdriver flag
    "evasions/navigator_vendor.js",      // Spoof vendor string
    "evasions/navigator_language.js",    // Language preferences
    "evasions/navigator_plugins.js",     // Plugin enumeration
    "evasions/navigator_permissions.js", // Permissions API
    // Hardware and UA fingerprinting
    "evasions/hardware_concurrency.js", // CPU core count spoofing
    "evasions/user_agent_data.js",      // Navigator.userAgentData API
    // Browser APIs and features
    "evasions/media_codecs.js",          // Media codec support
    "evasions/webgl_vendor_override.js", // WebGL fingerprinting
    "evasions/font_spoof.js",            // Font fingerprinting evasion
    "evasions/canvas_noise.js",          // Canvas fingerprinting protection (deterministic noise)
    // Window and frame behavior
    "evasions/window_outerdimensions.js", // Window measurements
    "iframe_content_window.js",           // IFrame handling
    // Chrome-specific APIs
    "evasions/chrome_app.js",     // Chrome app detection
    "evasions/chrome_runtime.js", // Runtime API
];

pub async fn inject(page: Page) -> Result<()> {
    // Generate per-session seed for canvas fingerprinting
    let session_seed: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();
    let session_seed_hex = hex::encode(&session_seed);

    info!("Injecting stealth scripts");

    let config = Config::default();

    let kromekover_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("kromekover");

    // Step 1: Inject window.grokConfig first (must happen before any scripts run)
    let grok_config = format!(
        r#"
        window.grokConfig = {{
            acceptLanguage: "{}",
            platform: "{}",
            language: "{}",
            languages: {},
            screenWidth: {},
            screenHeight: {},
            webglVendor: "{}",
            webglRenderer: "{}",
            hardwareConcurrency: {},
            sessionSeed: "{}"
        }};
        "#,
        config.accept_language,
        config.platform,
        config.language,
        serde_json::to_string(&config.languages).unwrap_or_else(|_| "[]".to_string()),
        config.screen_width,
        config.screen_height,
        config.webgl_vendor,
        config.webgl_renderer,
        config.hardware_concurrency,
        session_seed_hex,
    );

    info!("Injecting window.grokConfig");
    page.execute(
        cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams {
            source: grok_config,
            include_command_line_api: None,
            world_name: None,
            run_immediately: None,
        },
    )
    .await?;

    // Step 2: Load all script files with best-effort error handling
    info!("Loading {} evasion scripts", EVASION_SCRIPTS.len());

    let read_futures: Vec<_> = EVASION_SCRIPTS
        .iter()
        .map(|script| {
            let script_path = kromekover_dir.join(script);
            let script_name = (*script).to_string();
            async move {
                let source = fs::read_to_string(&script_path).await;
                (script_name, source)
            }
        })
        .collect();

    let read_results = join_all(read_futures).await;

    // Separate successes from failures with detailed logging
    let mut scripts = Vec::new();
    let mut failed_reads = Vec::new();

    for (script_name, result) in read_results {
        match result {
            Ok(source) => {
                debug!("✓ Loaded: {}", script_name);
                scripts.push((script_name, source));
            }
            Err(e) => {
                warn!("✗ Failed to load {}: {}", script_name, e);
                failed_reads.push(script_name);
            }
        }
    }

    info!(
        "Loaded {}/{} scripts successfully",
        scripts.len(),
        EVASION_SCRIPTS.len()
    );
    if !failed_reads.is_empty() {
        warn!(
            "Failed to load {} scripts: {:?}",
            failed_reads.len(),
            failed_reads
        );
    }

    // Step 4: Inject all loaded scripts with best-effort error handling
    info!("Injecting {} scripts", scripts.len());

    let inject_futures: Vec<_> = scripts
        .into_iter()
        .map(|(script_name, source)| {
            let page = page.clone();
            async move {
                let result = page
                    .execute(
                        cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams {
                            source,
                            include_command_line_api: None,
                            world_name: None,
                            run_immediately: None,
                        },
                    )
                    .await;
                (script_name, result)
            }
        })
        .collect();

    let inject_results = join_all(inject_futures).await;

    // Track injection success/failure with detailed logging
    let mut success_count = 0;
    let mut failed_injections = Vec::new();

    for (script_name, result) in inject_results {
        match result {
            Ok(_) => {
                debug!("✓ Injected: {}", script_name);
                success_count += 1;
            }
            Err(e) => {
                warn!("✗ Failed to inject {}: {}", script_name, e);
                failed_injections.push(script_name);
            }
        }
    }

    info!(
        "Successfully injected {}/{} scripts",
        success_count,
        EVASION_SCRIPTS.len()
    );
    if !failed_injections.is_empty() {
        warn!(
            "Failed to inject {} scripts: {:?}",
            failed_injections.len(),
            failed_injections
        );
    }

    // Check if any scripts succeeded - fail only if ZERO scripts were injected
    if success_count == 0 {
        return Err(anyhow::anyhow!(
            "Failed to inject any stealth scripts. Total failures: {} load, {} inject",
            failed_reads.len(),
            failed_injections.len()
        ));
    }

    // Step 6: Modify user agent last
    info!("Configuring user agent");
    let ua = page
        .execute(cdp::browser_protocol::browser::GetVersionParams {})
        .await?;

    let modified_ua = ua.user_agent.replace("Headless", "");

    page.execute(cdp::browser_protocol::network::SetUserAgentOverrideParams {
        user_agent: modified_ua,
        accept_language: Some(config.accept_language.clone()),
        platform: Some(config.platform.clone()),
        user_agent_metadata: None,
    })
    .await?;

    info!(
        "Stealth injection complete: {}/{} scripts active",
        success_count,
        EVASION_SCRIPTS.len()
    );
    Ok(())
}
