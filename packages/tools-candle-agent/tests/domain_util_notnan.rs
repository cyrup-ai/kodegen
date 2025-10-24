//! Tests for NotNaN utility module

use kodegen_candle_agent::domain::util::notnan::*;

#[test]
fn test_notnan_creation() {
    assert!(NotNan::new(1.0).is_ok());
    assert!(NotNan::new(0.0).is_ok());
    assert!(NotNan::new(-1.0).is_ok());
    assert!(NotNan::new(f32::INFINITY).is_ok());
    assert!(NotNan::new(f32::NEG_INFINITY).is_ok());
    assert!(NotNan::new(f32::NAN).is_err());
}

#[test]
fn test_notnan_ordering() -> Result<(), Box<dyn std::error::Error>> {
    let a = NotNan::new(1.0)?;
    let b = NotNan::new(2.0)?;
    let c = NotNan::new(1.0)?;

    assert!(a < b);
    assert!(b > a);
    assert_eq!(a, c);
    assert!(a <= c);
    assert!(a >= c);
    Ok(())
}

#[test]
fn test_notnan_into_inner() -> Result<(), Box<dyn std::error::Error>> {
    let val = std::f32::consts::PI;
    let not_nan = NotNan::new(val)?;
    assert!((not_nan.into_inner() - val).abs() < f32::EPSILON);
    Ok(())
}
