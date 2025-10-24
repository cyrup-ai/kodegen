use kodegen_simd::utils::{align_ptr, align_ptr_mut, simd_available};

#[test]
fn test_simd_available() {
    // Just verify it doesn't panic
    let _ = simd_available();
}

#[test]
fn test_align_ptr() {
    let array = [0u8; 64];
    let ptr = array.as_ptr();
    let aligned = align_ptr(ptr, 16);
    assert_eq!(aligned as usize % 16, 0);
}

#[test]
fn test_align_ptr_mut() {
    let mut array = [0u8; 64];
    let ptr = array.as_mut_ptr();
    let aligned = align_ptr_mut(ptr, 16);
    assert_eq!(aligned as usize % 16, 0);
}
