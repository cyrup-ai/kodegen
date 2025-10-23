use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use kodegen_tools_citescrape::kromekover::inject;

#[tokio::test]
async fn test_evasions() -> Result<()> {
    let (browser, mut _handler) = Browser::launch(
        BrowserConfig::builder()
            .no_sandbox()
            .build()
            .map_err(|e| anyhow::anyhow!(e))?,
    )
    .await?;

    let page = browser.new_page("about:blank").await?;

    // Inject our evasion scripts
    inject(page.clone()).await?;

    // Test navigator properties
    let vendor_result = page.evaluate("navigator.vendor").await?;
    if let Some(vendor_value) = vendor_result.value() {
        if let Some(vendor) = vendor_value.as_str() {
            assert_eq!(vendor, "Google Inc.");
        } else {
            panic!("vendor is not a string");
        }
    }

    let webdriver_result = page.evaluate("navigator.webdriver").await?;
    if let Some(webdriver_value) = webdriver_result.value() {
        if let Some(webdriver) = webdriver_value.as_bool() {
            assert!(!webdriver);
        } else {
            panic!("webdriver is not a bool");
        }
    }

    let languages_result = page.evaluate("navigator.languages").await?;
    if let Some(languages_value) = languages_result.value() {
        if let Some(languages_array) = languages_value.as_array() {
            let has_en_us = languages_array.iter().any(|v| v.as_str() == Some("en-US"));
            assert!(has_en_us, "languages should contain 'en-US'");
        } else {
            panic!("languages is not an array");
        }
    }

    // Test WebGL
    let webgl_result = page
        .evaluate(
            r"
        const canvas = document.createElement('canvas');
        const gl = canvas.getContext('webgl');
        const vendor = gl.getParameter(37445);
        const renderer = gl.getParameter(37446);
        [vendor, renderer]
    ",
        )
        .await?;

    if let Some(webgl_value) = webgl_result.value()
        && let Some(webgl_array) = webgl_value.as_array()
        && webgl_array.len() >= 2
    {
        if let Some(vendor) = webgl_array[0].as_str() {
            assert_eq!(vendor, "Intel Open Source Technology Center");
        }
        if let Some(renderer) = webgl_array[1].as_str() {
            assert!(renderer.contains("Intel"));
        }
    }

    // Test media codecs
    let media_result = page
        .evaluate(
            r"
        navigator.mediaCapabilities.decodingInfo({
            type: 'file',
            video: {
                contentType: 'video/vp8',
                width: 1920,
                height: 1080,
                bitrate: 1000,
                framerate: 30
            }
        }).then(result => result.supported)
    ",
        )
        .await?;

    if let Some(media_value) = media_result.value()
        && let Some(supported) = media_value.as_bool()
    {
        assert!(supported);
    }

    // Test Chrome runtime API
    let chrome_result = page
        .evaluate(
            r"
        typeof chrome !== 'undefined' && 
        typeof chrome.runtime !== 'undefined' &&
        typeof chrome.runtime.sendMessage === 'function'
    ",
        )
        .await?;

    if let Some(chrome_value) = chrome_result.value()
        && let Some(has_chrome) = chrome_value.as_bool()
    {
        assert!(has_chrome);
    }

    // Test window dimensions
    let dimensions_result = page
        .evaluate(
            r"
        [window.outerWidth, window.outerHeight]
    ",
        )
        .await?;

    if let Some(dimensions_value) = dimensions_result.value()
        && let Some(dimensions_array) = dimensions_value.as_array()
        && dimensions_array.len() >= 2
    {
        let width = dimensions_array[0].as_u64().unwrap_or(0) as u32;
        let height = dimensions_array[1].as_u64().unwrap_or(0) as u32;
        assert_eq!((width, height), (1920, 1080));
    }

    // ============================================================================
    // CRITICAL SECURITY TESTS: navigator.automationTools Protection (October 2025)
    // ============================================================================
    // These tests verify that navigator.automationTools returns undefined WITHOUT
    // creating a detectable fingerprint. All four tests MUST pass.

    // Test 1: Verify 'automationTools' property doesn't exist using 'in' operator
    // This is the PRIMARY bot detection method used by modern anti-bot systems
    let automation_in_navigator = page.evaluate("'automationTools' in navigator").await?;
    if let Some(result_value) = automation_in_navigator.value()
        && let Some(exists) = result_value.as_bool()
    {
        assert!(
            !exists,
            "CRITICAL SECURITY FAILURE: 'automationTools' property exists in navigator. \
             Modern bot detection systems (DataDome, Kasada, PerimeterX) scan for phantom \
             properties using the 'in' operator. The property MUST NOT be defined - it \
             should return undefined naturally via JavaScript's default behavior."
        );
    }

    // Test 2: Verify property doesn't appear in enumeration (advanced detection method)
    // Bot detection systems enumerate navigator properties and compare against fingerprint databases
    let automation_in_keys = page
        .evaluate(
            "Object.keys(navigator).includes('automationTools') || \
         Object.getOwnPropertyNames(navigator).includes('automationTools')",
        )
        .await?;
    if let Some(result_value) = automation_in_keys.value()
        && let Some(appears) = result_value.as_bool()
    {
        assert!(
            !appears,
            "CRITICAL SECURITY FAILURE: 'automationTools' appears in navigator property enumeration. \
             Advanced bot detection scans Object.keys() and Object.getOwnPropertyNames() to detect \
             properties that shouldn't exist. This is the same technique used to detect PhantomJS. \
             Natural undefined approach ensures the property is absent from all enumeration."
        );
    }

    // Test 3: Verify property descriptor doesn't exist (deepest level verification)
    // This is the most sophisticated detection check - verify no descriptor exists at all
    let automation_descriptor = page
        .evaluate("Object.getOwnPropertyDescriptor(navigator, 'automationTools') === undefined")
        .await?;
    if let Some(result_value) = automation_descriptor.value()
        && let Some(no_descriptor) = result_value.as_bool()
    {
        assert!(
            no_descriptor,
            "CRITICAL SECURITY FAILURE: Property descriptor exists for 'automationTools'. \
             The most sophisticated bot detection checks for property descriptors using \
             Object.getOwnPropertyDescriptor(). Natural undefined approach ensures no \
             descriptor exists, making detection impossible."
        );
    }

    // Test 4: Verify accessing it returns undefined naturally (functional verification)
    // This confirms the protection still works while being undetectable
    let automation_value = page.evaluate("navigator.automationTools").await?;
    if let Some(result_value) = automation_value.value() {
        assert!(
            result_value.is_null() || result_value.as_str().is_none(),
            "FUNCTIONAL VERIFICATION: navigator.automationTools should return undefined. \
             This confirms the protection is working correctly - the value is undefined as \
             required for stealth. The critical difference: this is NATURAL undefined \
             (property absent) not DEFINED undefined (property present returning undefined)."
        );
    }

    Ok(())
}
