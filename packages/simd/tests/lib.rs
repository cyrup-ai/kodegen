use kodegen_simd::{SIMD_WIDTH_8, VERSION};

#[test]
fn test_constants() {
    // Verify SIMD width is reasonable for vectorization
    assert_eq!(SIMD_WIDTH_8, 8);
    // Verify version string is populated from Cargo.toml
    assert!(VERSION.starts_with(char::is_numeric));
}
