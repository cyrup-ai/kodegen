use crate::linked_hash_map::LinkedHashMap;
use std::hash::{Hash, Hasher};

/// The YAML node representation, mirroring the original design:
/// - `Real` is an f64 stored as string (lazy parse).
/// - `Integer` is i64.
/// - `String` is an owned string.
/// - `Boolean` is bool.
/// - `Array` is a vector of `Yaml`.
/// - `Hash` is an insertion-order map (using `linked_hash_map` logic).
/// - `Alias` for referencing an anchor.
/// - `Null` represents explicit YAML null.
/// - `BadValue` is returned for invalid indexing or out-of-range lookups.
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord)]
pub enum Yaml {
    Real(String),
    Integer(i64),
    String(String),
    Boolean(bool),
    Array(Vec<Yaml>),
    Hash(LinkedHashMap<Yaml, Yaml>),
    Alias(usize),
    Tagged(String, Box<Yaml>),
    Null,
    BadValue,
}

impl Hash for Yaml {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Real(s) => {
                0.hash(state);
                s.hash(state);
            }
            Self::Integer(i) => {
                1.hash(state);
                i.hash(state);
            }
            Self::String(s) => {
                2.hash(state);
                s.hash(state);
            }
            Self::Boolean(b) => {
                3.hash(state);
                b.hash(state);
            }
            Self::Array(a) => {
                4.hash(state);
                a.hash(state);
            }
            Self::Hash(h) => {
                5.hash(state);
                h.hash(state);
            }
            Self::Alias(i) => {
                6.hash(state);
                i.hash(state);
            }
            Self::Tagged(tag, value) => {
                7.hash(state);
                tag.hash(state);
                value.hash(state);
            }
            Self::Null => {
                8.hash(state);
            }
            Self::BadValue => {
                9.hash(state);
            }
        }
    }
}

/// This `BadValue` is used if we do `doc["unknown"]`, so indexing is graceful.
static BAD_VALUE: Yaml = Yaml::BadValue;

/// Accessors for Yaml
impl Yaml {
    #[inline(always)]
    #[must_use] 
    pub const fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Boolean(b) => Some(b),
            _ => None,
        }
    }

    #[inline(always)]
    #[must_use] 
    pub const fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Integer(i) => Some(i),
            _ => None,
        }
    }

    #[inline]
    #[must_use] 
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Real(ref s) => parse_f64(s),
            _ => None,
        }
    }

    #[inline(always)]
    #[must_use] 
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Self::String(ref s) => Some(s),
            _ => None,
        }
    }

    #[inline(always)]
    #[must_use] 
    pub fn as_vec(&self) -> Option<&[Self]> {
        match *self {
            Self::Array(ref v) => Some(v),
            _ => None,
        }
    }

    #[inline(always)]
    #[must_use] 
    pub const fn as_hash(&self) -> Option<&LinkedHashMap<Self, Self>> {
        match *self {
            Self::Hash(ref h) => Some(h),
            _ => None,
        }
    }

    #[inline(always)]
    #[must_use] 
    pub const fn is_null(&self) -> bool {
        matches!(*self, Self::Null)
    }

    #[inline(always)]
    #[must_use] 
    pub const fn is_badvalue(&self) -> bool {
        matches!(*self, Self::BadValue)
    }

    /// Parse a string into a Yaml value with automatic type detection
    #[inline]
    #[must_use] 
    pub fn parse_str(v: &str) -> Self {
        // Handle hexadecimal numbers (0x, +0x, -0x)
        if let Some(stripped) = v.strip_prefix("0x")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c.is_ascii_hexdigit())
            && let Ok(i) = i64::from_str_radix(stripped, 16)
        {
            return Self::Integer(i);
        }
        if let Some(stripped) = v.strip_prefix("+0x")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c.is_ascii_hexdigit())
            && let Ok(i) = i64::from_str_radix(stripped, 16)
        {
            return Self::Integer(i);
        }
        if let Some(stripped) = v.strip_prefix("-0x")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c.is_ascii_hexdigit())
            && let Ok(i) = i64::from_str_radix(stripped, 16)
        {
            return Self::Integer(-i);
        }
        // Handle octal numbers (0o, +0o, -0o)
        if let Some(stripped) = v.strip_prefix("0o")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c.is_ascii_digit() && c < '8')
            && let Ok(i) = i64::from_str_radix(stripped, 8)
        {
            return Self::Integer(i);
        }
        if let Some(stripped) = v.strip_prefix("+0o")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c.is_ascii_digit() && c < '8')
            && let Ok(i) = i64::from_str_radix(stripped, 8)
        {
            return Self::Integer(i);
        }
        if let Some(stripped) = v.strip_prefix("-0o")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c.is_ascii_digit() && c < '8')
            && let Ok(i) = i64::from_str_radix(stripped, 8)
        {
            return Self::Integer(-i);
        }
        // Handle binary numbers (0b, +0b, -0b)
        if let Some(stripped) = v.strip_prefix("0b")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c == '0' || c == '1')
            && let Ok(i) = i64::from_str_radix(stripped, 2)
        {
            return Self::Integer(i);
        }
        if let Some(stripped) = v.strip_prefix("+0b")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c == '0' || c == '1')
            && let Ok(i) = i64::from_str_radix(stripped, 2)
        {
            return Self::Integer(i);
        }
        if let Some(stripped) = v.strip_prefix("-0b")
            && !stripped.is_empty()
            && stripped.chars().all(|c| c == '0' || c == '1')
            && let Ok(i) = i64::from_str_radix(stripped, 2)
        {
            return Self::Integer(-i);
        }
        if let Some(stripped) = v.strip_prefix('+')
            && let Ok(i) = stripped.parse::<i64>()
            && !has_invalid_leading_zeros(v)
            && !has_invalid_sign_prefix(v)
        {
            return Self::Integer(i);
        }
        match v {
            "~" | "null" => Self::Null,
            "true" => Self::Boolean(true),
            "false" => Self::Boolean(false),
            _ if v.parse::<i64>().is_ok()
                && !has_invalid_leading_zeros(v)
                && !has_invalid_sign_prefix(v) =>
            {
                if let Ok(i) = v.parse::<i64>() {
                    Self::Integer(i)
                } else {
                    Self::String(v.into())
                }
            }
            _ if parse_f64(v).is_some() => Self::Real(v.into()),
            _ => Self::String(v.into()),
        }
    }
}

/// Check if a string has invalid sign prefixes (++, +-, -+, --)
fn has_invalid_sign_prefix(v: &str) -> bool {
    v.starts_with("++") || v.starts_with("+-") || v.starts_with("-+") || v.starts_with("--")
}

/// Check if a string contains invalid leading zeros for YAML 1.2 integer parsing
/// In YAML 1.2, integers with leading zeros should be treated as strings
/// unless they use explicit base prefixes (0x, 0o, 0b)
fn has_invalid_leading_zeros(v: &str) -> bool {
    // Allow single zero
    if v == "0" || v == "+0" || v == "-0" {
        return false;
    }

    // Check for leading zeros in positive numbers
    if v.starts_with('0') && v.len() > 1 && v.chars().nth(1).unwrap().is_ascii_digit() {
        return true;
    }

    // Check for leading zeros in signed numbers
    if (v.starts_with("+0") || v.starts_with("-0"))
        && v.len() > 2
        && v.chars().nth(2).unwrap().is_ascii_digit()
    {
        return true;
    }

    false
}

/// Convert string to float (including .inf, .nan).
pub fn parse_f64(v: &str) -> Option<f64> {
    match v {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Some(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => Some(f64::NAN),
        _ => {
            // Reject strings with invalid leading zeros or sign prefixes for YAML 1.2 compliance
            if has_invalid_leading_zeros(v) || has_invalid_sign_prefix(v) {
                None
            } else {
                v.parse::<f64>().ok()
            }
        }
    }
}

/// Indexing by &str
impl std::ops::Index<&str> for Yaml {
    type Output = Self;
    #[inline]
    fn index(&self, idx: &str) -> &Self {
        let key = Self::String(idx.to_owned());
        match self.as_hash() {
            Some(h) => h.get(&key).unwrap_or(&BAD_VALUE),
            None => &BAD_VALUE,
        }
    }
}

/// Indexing by usize
impl std::ops::Index<usize> for Yaml {
    type Output = Self;
    #[inline]
    fn index(&self, idx: usize) -> &Self {
        if let Some(v) = self.as_vec() {
            v.get(idx).unwrap_or(&BAD_VALUE)
        } else if let Some(h) = self.as_hash() {
            let key = Self::Integer(idx as i64);
            h.get(&key).unwrap_or(&BAD_VALUE)
        } else {
            &BAD_VALUE
        }
    }
}
