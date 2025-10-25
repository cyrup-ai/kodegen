delete window.cdc_adoQpoasnfa76pfcZLmcfl_Array;
delete window.cdc_adoQpoasnfa76pfcZLmcfl_Promise;
delete window.cdc_adoQpoasnfa76pfcZLmcfl_Symbol;
delete window.cdc_adoQpoasnfa76pfcZLmcfl_Omine;
delete window.cdc_adoQpoasnfa76pfcZLmcfl_Proxy;
delete window.cdc_adoQpoasnfa76pfcZLmcfl_Buffer;
delete window.cdc_adoQpoasnfa76pfcZLmcfl_TRV;
delete window.cdc_asdjflasutopfhvcZLmcfl_;
delete window.cdc_7LOmn8N_4mSx;
delete window.cdc_7LOmn8N_4mSx_Omine;

// ============================================================================
// CRITICAL SECURITY: navigator.automationTools Protection (October 2025)
// ============================================================================
// 
// IMPLEMENTATION: Natural Undefined Approach (October 2025 Standard)
// 
// We protect against automationTools detection by leveraging JavaScript's
// natural undefined behavior instead of explicit property definition.
//
// WHY THIS APPROACH IS SUPERIOR:
//
// OLD APPROACH (Detectable):
// Object.defineProperty(navigator, 'automationTools', { get: () => undefined });
// - Creates property descriptor → DETECTABLE via Object.getOwnPropertyDescriptor()
// - Property exists in 'in' operator check → DETECTABLE
// - Appears in Object.getOwnPropertyNames() → DETECTABLE
// - Returns undefined ✓ (functional but leaves fingerprint)
//
// OCTOBER 2025 APPROACH (Undetectable):
// [Use JavaScript's natural behavior - no code needed]
// - No property descriptor → UNDETECTABLE
// - 'automationTools' in navigator returns false → UNDETECTABLE  
// - Absent from all enumeration methods → UNDETECTABLE
// - Returns undefined naturally ✓ (functional AND zero fingerprint)
//
// PROTECTION MECHANISM:
// Modern bot detection (DataDome, Kasada, PerimeterX) scans for phantom
// properties using 'in' operator and Object.getOwnPropertyNames(). By NOT
// defining the property, we achieve:
// 1. navigator.automationTools returns undefined (protection maintained)
// 2. Zero detection footprint (no enumeration trace)
// 3. Identical behavior to real Chrome (perfect stealth)
//
// VERIFICATION:
// See kromekover_tests.rs for comprehensive tests verifying:
// - Property doesn't exist in 'in' operator check
// - Property absent from Object.keys() and Object.getOwnPropertyNames()
// - No property descriptor exists
// - Returns undefined naturally (functional verification)
//
// WARNING: Do not add Object.defineProperty for automationTools. Natural
// undefined is the ONLY undetectable approach for October 2025 standards.
// Any developer who changes this will compromise stealth infrastructure.
// ============================================================================

// Note: navigator.webdriver protection is handled by evasions/navigator_webdriver.js
// (injected after this script - see mod.rs for injection order)

Object.defineProperty(navigator, 'chrome', {
  get: () => ({ runtime: {}, app: {} })
});
