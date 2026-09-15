//! Grammar this module accepts, one line at a time:
//!
//!   TIMESTAMP SP LEVEL SP MODULE ':' SP MESSAGE (' | ' FIELDS)?
//!
//! TIMESTAMP is a fixed-width "YYYY-MM-DDTHH:MM:SS.mmmZ" (24 bytes, always UTC).
//! LEVEL is one of TRACE, DEBUG, INFO, WARN, ERROR.
//! MODULE is one or more '.'-separated identifiers (letters, digits, underscore).
//! MESSAGE is free text and may not itself contain the literal " | " marker,
//! since that is what introduces the field list.
//! FIELDS is space-separated key=value pairs; a value with whitespace or a
//! double quote must be wrapped in double quotes, with \" and \\ as escapes.
//!
//! In strict mode every deviation from this grammar is a hard error. In
//! lenient mode the parser recovers as much structure as it can instead of
//! giving up on the whole line.

use std::fmt;

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn from_str(s: &str) -> Option<Level> {
        match s {
            "TRACE" => Some(Level::Trace),
            "DEBUG" => Some(Level::Debug),
            "INFO" => Some(Level::Info),
            "WARN" => Some(Level::Warn),
            "ERROR" => Some(Level::Error),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

#[derive(Debug)]
pub enum LevelValue {
    Known(Level),
    Unknown(String),
}

impl LevelValue {
    pub fn as_str(&self) -> &str {
        match self {
            LevelValue::Known(l) => l.as_str(),
            LevelValue::Unknown(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Timestamp {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millis: u16,
}

impl Timestamp {
    fn is_leap_year(year: u16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    fn days_in_month(year: u16, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.month < 1 || self.month > 12 {
            return Err(format!("month {} out of range", self.month));
        }
        let max_day = Self::days_in_month(self.year, self.month);
        if self.day < 1 || self.day > max_day {
            return Err(format!(
                "day {} out of range for {:04}-{:02}",
                self.day, self.year, self.month
            ));
        }
        if self.hour > 23 {
            return Err(format!("hour {} out of range", self.hour));
        }
        if self.minute > 59 {
            return Err(format!("minute {} out of range", self.minute));
        }
        if self.second > 59 {
            return Err(format!("second {} out of range", self.second));
        }
        Ok(())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.millis
        )
    }
}

#[derive(Debug)]
pub struct LogEntry {
    pub timestamp: Option<Timestamp>,
    pub level: LevelValue,
    pub module: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
}

pub fn parse_line(line: &str, lenient: bool) -> Result<LogEntry, ParseError> {
    let (ts_str, rest) = split_token(line, lenient)?;
    let timestamp = match parse_timestamp(ts_str) {
        Ok(t) => Some(t),
        Err(e) => {
            if lenient {
                None
            } else {
                return Err(ParseError(format!("invalid timestamp '{}': {}", ts_str, e)));
            }
        }
    };

    let (level_str, rest) = split_token(rest, lenient)?;
    let level = match Level::from_str(level_str) {
        Some(l) => LevelValue::Known(l),
        None => {
            if lenient {
                LevelValue::Unknown(level_str.to_string())
            } else {
                return Err(ParseError(format!(
                    "unknown level '{}', expected one of TRACE, DEBUG, INFO, WARN, ERROR",
                    level_str
                )));
            }
        }
    };

    let (module_token, rest) = split_token(rest, lenient)?;
    let module = if let Some(stripped) = module_token.strip_suffix(':') {
        if !lenient {
            validate_module(stripped)?;
        }
        stripped.to_string()
    } else if lenient {
        module_token.to_string()
    } else {
        return Err(ParseError(format!(
            "expected module name followed by ':', found '{}'",
            module_token
        )));
    };

    let (message, fields) = split_message_and_fields(rest, lenient)?;

    Ok(LogEntry {
        timestamp,
        level,
        module,
        message,
        fields,
    })
}

fn split_token<'a>(s: &'a str, lenient: bool) -> Result<(&'a str, &'a str), ParseError> {
    if s.is_empty() {
        return Err(ParseError(
            "unexpected end of line, expected another field".to_string(),
        ));
    }
    if !lenient && s.starts_with(' ') {
        return Err(ParseError("unexpected extra whitespace".to_string()));
    }
    let s = if lenient { s.trim_start_matches(' ') } else { s };
    match s.find(' ') {
        Some(idx) => {
            let token = &s[..idx];
            let remainder = &s[idx + 1..];
            if token.is_empty() {
                return Err(ParseError("unexpected extra whitespace".to_string()));
            }
            Ok((token, remainder))
        }
        None => Err(ParseError("line ended too early".to_string())),
    }
}

fn parse_timestamp(s: &str) -> Result<Timestamp, String> {
    let bytes = s.as_bytes();
    if bytes.len() != 24 {
        return Err(format!(
            "timestamp must be exactly 24 characters (YYYY-MM-DDTHH:MM:SS.mmmZ), got {}",
            bytes.len()
        ));
    }

    let digit = |i: usize| -> Result<u32, String> {
        let b = bytes[i];
        if b.is_ascii_digit() {
            Ok((b - b'0') as u32)
        } else {
            Err(format!(
                "expected digit at position {}, found '{}'",
                i, b as char
            ))
        }
    };
    let expect = |i: usize, ch: u8| -> Result<(), String> {
        if bytes[i] == ch {
            Ok(())
        } else {
            Err(format!(
                "expected '{}' at position {}, found '{}'",
                ch as char, i, bytes[i] as char
            ))
        }
    };

    let year = digit(0)? * 1000 + digit(1)? * 100 + digit(2)? * 10 + digit(3)?;
    expect(4, b'-')?;
    let month = digit(5)? * 10 + digit(6)?;
    expect(7, b'-')?;
    let day = digit(8)? * 10 + digit(9)?;
    expect(10, b'T')?;
    let hour = digit(11)? * 10 + digit(12)?;
    expect(13, b':')?;
    let minute = digit(14)? * 10 + digit(15)?;
    expect(16, b':')?;
    let second = digit(17)? * 10 + digit(18)?;
    expect(19, b'.')?;
    let millis = digit(20)? * 100 + digit(21)? * 10 + digit(22)?;
    expect(23, b'Z')?;

    let ts = Timestamp {
        year: year as u16,
        month: month as u8,
        day: day as u8,
        hour: hour as u8,
        minute: minute as u8,
        second: second as u8,
        millis: millis as u16,
    };
    ts.validate()?;
    Ok(ts)
}

fn split_message_and_fields(
    rest: &str,
    lenient: bool,
) -> Result<(String, Vec<(String, String)>), ParseError> {
    let (message_part, fields_part) = match rest.find(" | ") {
        Some(idx) => (&rest[..idx], Some(&rest[idx + 3..])),
        None => (rest, None),
    };

    if message_part.is_empty() && !lenient {
        return Err(ParseError("message must not be empty".to_string()));
    }

    if !lenient {
        for c in message_part.chars() {
            if c.is_control() {
                return Err(ParseError(format!(
                    "message contains control character {:?}",
                    c
                )));
            }
        }
    }

    let mut fields: Vec<(String, String)> = Vec::new();
    if let Some(fields_str) = fields_part {
        for token in tokenize_fields(fields_str, lenient)? {
            match parse_field(token) {
                Ok((key, value)) => {
                    if fields.iter().any(|(k, _)| k == &key) {
                        if lenient {
                            fields.retain(|(k, _)| k != &key);
                        } else {
                            return Err(ParseError(format!("duplicate field key '{}'", key)));
                        }
                    }
                    fields.push((key, value));
                }
                Err(e) => {
                    if lenient {
                        continue;
                    }
                    return Err(ParseError(e));
                }
            }
        }
    }

    Ok((message_part.to_string(), fields))
}

fn tokenize_fields(s: &str, lenient: bool) -> Result<Vec<&str>, ParseError> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        while i < len && bytes[i] == b' ' {
            i += 1;
        }
        if i >= len {
            break;
        }
        let start = i;
        let mut in_quotes = false;
        while i < len {
            let b = bytes[i];
            if b == b'"' {
                in_quotes = !in_quotes;
                i += 1;
            } else if b == b'\\' && in_quotes && i + 1 < len && (bytes[i + 1] == b'"' || bytes[i + 1] == b'\\') {
                i += 2;
            } else if b == b' ' && !in_quotes {
                break;
            } else {
                i += 1;
            }
        }
        if in_quotes && !lenient {
            return Err(ParseError(format!(
                "unterminated quoted value starting at '{}'",
                &s[start..i]
            )));
        }
        tokens.push(&s[start..i]);
    }

    Ok(tokens)
}

fn parse_field(token: &str) -> Result<(String, String), String> {
    let eq_idx = token
        .find('=')
        .ok_or_else(|| format!("field '{}' is missing '='", token))?;
    let key = &token[..eq_idx];
    let raw_value = &token[eq_idx + 1..];

    validate_identifier(key).map_err(|e| format!("invalid field key '{}': {}", key, e))?;

    let value = if raw_value.starts_with('"') {
        if raw_value.len() < 2 || !raw_value.ends_with('"') {
            return Err(format!(
                "value for field '{}' has an unterminated quote",
                key
            ));
        }
        unescape(&raw_value[1..raw_value.len() - 1])
    } else {
        if raw_value.contains('"') {
            return Err(format!("value for field '{}' contains a stray quote", key));
        }
        raw_value.to_string()
    };

    Ok((key.to_string(), value))
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn validate_identifier(s: &str) -> Result<(), String> {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        Some(c) => return Err(format!("must start with a letter or underscore, found '{}'", c)),
        None => return Err("must not be empty".to_string()),
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("invalid character '{}'", c));
        }
    }
    Ok(())
}

fn validate_module(s: &str) -> Result<(), ParseError> {
    if s.is_empty() {
        return Err(ParseError("module name must not be empty".to_string()));
    }
    for segment in s.split('.') {
        validate_identifier(segment)
            .map_err(|e| ParseError(format!("invalid module segment '{}' in '{}': {}", segment, s, e)))?;
    }
    Ok(())
}
