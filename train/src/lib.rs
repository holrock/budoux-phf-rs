//! A Rust port of BudouX's model training pipeline.
//!
//! Each module transcribes one of the upstream scripts in
//! [`budoux/scripts`](https://github.com/google/budoux/tree/main/scripts):
//!
//! | Module | Upstream script | Input → output |
//! |--------|-----------------|----------------|
//! | [`encode`] | `encode_data.py` | segmented source text → encoded data |
//! | [`adaboost`] | `train.py` | encoded data → weights |
//! | [`model`] | `build_model.py` | weights → model JSON |
//!
//! The file formats are the upstream ones, so every stage can be mixed with
//! the Python scripts, and the model JSON is what `codegen` turns into a
//! `model_*.rs` file.

use std::io::{self, BufRead};

pub mod adaboost;
pub mod encode;
pub mod model;

/// Whether `c` is whitespace to Python's `str.isspace`, which is what the
/// upstream scripts' `str.strip()` trims.
///
/// That is Rust's `White_Space` plus the four ASCII information separators
/// U+001C..=U+001F, which Python counts as whitespace and Rust does not.
fn is_py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python's `str.strip()`.
pub(crate) fn py_strip(s: &str) -> &str {
    s.trim_matches(is_py_space)
}

/// Calls `f` on every line of `reader`, splitting lines the way Python's text
/// mode does (universal newlines: `\n`, `\r\n` and a lone `\r` all end a line).
///
/// `f` also receives the 1-based line number, for error messages. A line that
/// is not valid UTF-8 is an error.
pub(crate) fn for_each_line<R: BufRead>(
    mut reader: R,
    mut f: impl FnMut(usize, &str) -> io::Result<()>,
) -> io::Result<()> {
    let mut buf = Vec::new();
    let mut lineno = 0;
    loop {
        buf.clear();
        if reader.read_until(b'\n', &mut buf)? == 0 {
            return Ok(());
        }
        let chunk = buf.strip_suffix(b"\n").unwrap_or(&buf);
        // A `\r` ends a line too. `\r\n` leaves an empty piece behind, which is
        // harmless: every caller skips blank lines.
        for piece in chunk.split(|&b| b == b'\r') {
            lineno += 1;
            let line = std::str::from_utf8(piece).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("line {lineno}: invalid UTF-8: {e}"),
                )
            })?;
            f(lineno, line)?;
        }
    }
}

pub(crate) fn invalid_data(lineno: usize, msg: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("line {lineno}: {msg}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_strip_matches_python() {
        assert_eq!(py_strip(" \t\u{3000}a b\u{1f}\r\n"), "a b");
        assert_eq!(py_strip("\u{2581}a\u{2581}"), "\u{2581}a\u{2581}");
    }

    #[test]
    fn universal_newlines() {
        let mut lines = Vec::new();
        for_each_line("a\r\nb\rc\nd".as_bytes(), |_, l| {
            lines.push(l.to_string());
            Ok(())
        })
        .unwrap();
        let lines: Vec<_> = lines.into_iter().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines, ["a", "b", "c", "d"]);
    }
}
