use crate::linked_hash_map::LinkedHashMap;
use crate::yaml::Yaml;
use std::error::Error;
use std::fmt;

/// An Emitter for Yaml => String, with anchors etc.
pub struct YamlEmitter<'a> {
    writer: &'a mut dyn fmt::Write,
    pub best_indent: usize,
    pub compact: bool,
    level: isize,
}

#[derive(Debug)]
pub enum EmitError {
    FmtError(fmt::Error),
    BadHashmapKey,
}

impl From<fmt::Error> for EmitError {
    fn from(e: fmt::Error) -> Self {
        Self::FmtError(e)
    }
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FmtError(e) => write!(f, "format error: {e}"),
            Self::BadHashmapKey => write!(f, "bad hashmap key"),
        }
    }
}

impl Error for EmitError {}

pub type EmitResult = Result<(), EmitError>;

impl<'a> YamlEmitter<'a> {
    pub fn new(writer: &'a mut dyn fmt::Write) -> Self {
        YamlEmitter {
            writer,
            best_indent: 2,
            compact: true,
            level: -1,
        }
    }

    pub fn dump(&mut self, doc: &Yaml) -> EmitResult {
        writeln!(self.writer, "---")?;
        self.level = -1;
        self.emit_node(doc)?;
        Ok(())
    }

    pub fn emit(&mut self, doc: &Yaml) -> EmitResult {
        self.level = -1;
        self.emit_node(doc)?;
        Ok(())
    }

    fn emit_node(&mut self, node: &Yaml) -> EmitResult {
        match node {
            Yaml::Array(v) => self.emit_array(v),
            Yaml::Hash(h) => self.emit_hash(h),
            Yaml::String(s) => {
                if need_quotes(s) {
                    escape_str(self.writer, s)?;
                } else {
                    write!(self.writer, "{s}")?;
                }
                Ok(())
            }
            Yaml::Boolean(b) => {
                write!(self.writer, "{}", if *b { "true" } else { "false" })?;
                Ok(())
            }
            Yaml::Integer(i) => {
                write!(self.writer, "{i}")?;
                Ok(())
            }
            Yaml::Real(s) => {
                write!(self.writer, "{s}")?;
                Ok(())
            }
            Yaml::Null | Yaml::BadValue => {
                write!(self.writer, "~")?;
                Ok(())
            }
            Yaml::Alias(_) => {
                // If we had anchor references, we'd store them. For demonstration, we skip.
                Ok(())
            }
            Yaml::Tagged(tag, value) => {
                // Emit tagged value with tag prefix
                write!(self.writer, "{} ", tag)?;
                self.emit_node(value)
            }
        }
    }

    fn emit_array(&mut self, arr: &[Yaml]) -> EmitResult {
        if arr.is_empty() {
            write!(self.writer, "[]")?;
        } else {
            self.level += 1;
            for (i, val) in arr.iter().enumerate() {
                if i > 0 {
                    writeln!(self.writer)?;
                    self.write_indent()?;
                }
                write!(self.writer, "- ")?;
                self.emit_val(true, val)?;
            }
            self.level -= 1;
        }
        Ok(())
    }

    fn emit_hash(&mut self, h: &LinkedHashMap<Yaml, Yaml>) -> EmitResult {
        if h.is_empty() {
            write!(self.writer, "{{}}")?;
        } else {
            self.level += 1;
            let mut first = true;
            for (k, v) in h.iter() {
                if !first {
                    writeln!(self.writer)?;
                    self.write_indent()?;
                } else {
                    first = false;
                }
                if matches!(k, Yaml::Array(_) | Yaml::Hash(_)) {
                    // complex key
                    write!(self.writer, "? ")?;
                    self.emit_node(k)?;
                    writeln!(self.writer)?;
                    self.write_indent()?;
                    write!(self.writer, ": ")?;
                    self.emit_val(true, v)?;
                } else {
                    self.emit_node(k)?;
                    write!(self.writer, ": ")?;
                    self.emit_val(false, v)?;
                }
            }
            self.level -= 1;
        }
        Ok(())
    }

    fn emit_val(&mut self, inline: bool, val: &Yaml) -> EmitResult {
        match val {
            Yaml::Array(a) => {
                if (inline && self.compact) || a.is_empty() {
                    write!(self.writer, "")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_array(a)
            }
            Yaml::Hash(h) => {
                if (inline && self.compact) || h.is_empty() {
                    write!(self.writer, "")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_hash(h)
            }
            _ => self.emit_node(val),
        }
    }

    fn write_indent(&mut self) -> EmitResult {
        if self.level <= 0 {
            return Ok(());
        }
        for _ in 0..self.level {
            for _ in 0..self.best_indent {
                write!(self.writer, " ")?;
            }
        }
        Ok(())
    }
}

/// Return whether a string definitely needs quotes in YAML.
fn need_quotes(s: &str) -> bool {
    fn need_quotes_spaces(s: &str) -> bool {
        s.starts_with(' ') || s.ends_with(' ')
    }
    if s.is_empty() {
        return true;
    }
    if need_quotes_spaces(s) {
        return true;
    }
    if s.parse::<i64>().is_ok() {
        return true;
    }
    if s.parse::<f64>().is_ok() {
        return true;
    }
    match s {
        "null" | "~" | "NULL" | "Null" => return true,
        "true" | "false" | "True" | "False" => return true,
        _ => {}
    }
    // check special chars
    if s.starts_with(|c: char| {
        matches!(
            c,
            ':' | '&' | '*' | '?' | '|' | '-' | '<' | '>' | '=' | '!' | '%' | '@'
        )
    }) || s.contains(|c: char| {
        matches!(
            c,
            '{' | '}' | '[' | ']' | ',' | '#' | '`' | '\"' | '\''
                | '\\'
                | '\0'..='\x06'
                | '\t'
                | '\n'
                | '\r'
                | '\x0e'..='\x1f'
        )
    }) {
        return true;
    }
    false
}

/// Escape a string for double-quoted YAML
fn escape_str(wr: &mut dyn fmt::Write, s: &str) -> Result<(), fmt::Error> {
    write!(wr, "\"")?;
    for c in s.chars() {
        match c {
            '"' => write!(wr, "\\\"")?,
            '\\' => write!(wr, "\\\\")?,
            '\n' => write!(wr, "\\n")?,
            '\t' => write!(wr, "\\t")?,
            '\r' => write!(wr, "\\r")?,
            _ if c.is_control() => {
                // escape in \u form
                write!(wr, "\\u{:04x}", c as u32)?
            }
            _ => write!(wr, "{c}")?,
        }
    }
    write!(wr, "\"")?;
    Ok(())
}
