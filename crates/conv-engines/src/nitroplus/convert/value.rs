//! Values at convert time and the words and colours scripts name.

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Value {
    Null,
    Int(i64),
    Str(String),
    /// `@n`: relative to the current value.
    Rel(i64),
    /// A story flag whose value is only known when the game runs: it lives
    /// in this NScripter variable.
    Runtime(conv_core::ir::Var),
}

impl Value {
    pub(super) fn truthy(&self) -> bool {
        match self {
            Value::Null | Value::Runtime(_) => false,
            Value::Int(n) | Value::Rel(n) => *n != 0,
            Value::Str(s) => !s.is_empty(),
        }
    }

    pub(super) fn int(&self) -> i64 {
        match self {
            Value::Null | Value::Runtime(_) => 0,
            Value::Int(n) | Value::Rel(n) => *n,
            Value::Str(s) => s.trim().parse().unwrap_or(0),
        }
    }

    pub(super) fn string(&self) -> String {
        match self {
            Value::Null | Value::Runtime(_) => String::new(),
            Value::Int(n) | Value::Rel(n) => n.to_string(),
            Value::Str(s) => s.clone(),
        }
    }

    pub(super) fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Runtime(_), _) | (_, Value::Runtime(_)) => false,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Null, Value::Str(s)) | (Value::Str(s), Value::Null) => s.is_empty(),
            (Value::Str(s), n) | (n, Value::Str(s)) => s.trim().parse::<i64>().is_ok_and(|v| v == n.int()),
            (a, b) => a.int() == b.int(),
        }
    }

    /// A position argument: a number, `@n`, or a name such as `Center` for
    /// an object `size` px wide on a `screen` px axis.
    pub(super) fn position(&self, current: i64, size: i64, screen: i64) -> i64 {
        match self {
            Value::Int(n) => *n,
            Value::Rel(d) => current + d,
            Value::Null | Value::Runtime(_) => current,
            Value::Str(s) => match s.to_ascii_lowercase().as_str() {
                "center" | "middle" => (screen - size) / 2,
                "inleft" | "left" | "intop" | "top" => 0,
                "inright" | "right" | "inbottom" | "bottom" => screen - size,
                "outleft" | "outtop" => -size,
                "outright" | "outbottom" => screen,
                "onleft" | "ontop" => -size / 2,
                "onright" | "onbottom" => screen - size / 2,
                _ => s.trim().parse().unwrap_or(current),
            },
        }
    }
}

pub(super) fn word(w: &str) -> Value {
    match w.to_ascii_lowercase().as_str() {
        "true" => Value::Int(1),
        "false" => Value::Int(0),
        "null" => Value::Null,
        _ => Value::Str(w.to_string()),
    }
}

pub(super) fn color(v: &Value) -> Option<u32> {
    match v {
        Value::Int(n) => Some(*n as u32 & 0xFF_FFFF),
        // src: Muramasa.exe FUN_00414dd0 (upper-cased by FUN_00414980), Django.exe
        // 00410030: the HTML 4 names, GREEN 0x008000 (LIME is 0x00FF00).
        Value::Str(s) => Some(match s.to_ascii_lowercase().as_str() {
            "black" => 0x000000,
            "gray" => 0x808080,
            "silver" => 0xC0C0C0,
            "white" => 0xFFFFFF,
            "red" => 0xFF0000,
            "lime" => 0x00FF00,
            "blue" => 0x0000FF,
            "yellow" => 0xFFFF00,
            "aqua" => 0x00FFFF,
            "fuchsia" => 0xFF00FF,
            "maroon" => 0x800000,
            "green" => 0x008000,
            "navy" => 0x000080,
            "olive" => 0x808000,
            "teal" => 0x008080,
            "purple" => 0x800080,
            other => u32::from_str_radix(other.trim_start_matches('#'), 16).ok()?,
        }),
        _ => None,
    }
}

pub(super) fn add(l: &Value, r: &Value) -> Value {
    match (l, r) {
        (Value::Str(_), _) | (_, Value::Str(_)) => Value::Str(l.string() + &r.string()),
        _ => Value::Int(l.int() + r.int()),
    }
}
