//! Coded errors: the UI translates "code\u{1f}arg0\u{1f}arg1" (see src/lib/i18n).
use std::fmt::Display;

pub fn ec(code: &str, args: &[&dyn Display]) -> String {
    let mut s = code.to_string();
    for a in args { s.push('\u{1f}'); s.push_str(&a.to_string()); }
    s
}
