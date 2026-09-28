//! Port of `encode_data.py`: turns segmented source text into encoded data.
//!
//! The source text marks every segment boundary with [`SEP`] (`▁`, U+2581),
//! and a line break is a boundary too:
//!
//! ```text
//! 今日は▁良い▁天気ですね。
//! 明日も▁天気でしょう。
//! ```
//!
//! Every character becomes one line of the output: `1` (a boundary follows it)
//! or `-1` (it does not), times `scale`, then the features around it,
//! tab-separated.

use std::collections::HashSet;
use std::io::{self, Write};

/// The separator marking a segment boundary in source text.
pub const SEP: char = '\u{2581}';

/// The placeholder for a character position outside the sentence. A feature
/// containing it is dropped.
pub const INVALID: char = '\u{2594}';

/// Returns the features for the boundary between `w3` and `w4`, in upstream's
/// order. `w1`..`w6` are the three characters on either side, or [`INVALID`].
pub fn get_feature(w: [char; 6]) -> Vec<String> {
    let [w1, w2, w3, w4, w5, w6] = w;
    let raw: [(&str, &[char]); 13] = [
        ("UW1", &[w1]),
        ("UW2", &[w2]),
        ("UW3", &[w3]),
        ("UW4", &[w4]),
        ("UW5", &[w5]),
        ("UW6", &[w6]),
        ("BW1", &[w2, w3]),
        ("BW2", &[w3, w4]),
        ("BW3", &[w4, w5]),
        ("TW1", &[w1, w2, w3]),
        ("TW2", &[w2, w3, w4]),
        ("TW3", &[w3, w4, w5]),
        ("TW4", &[w4, w5, w6]),
    ];
    raw.iter()
        .filter(|(_, chars)| !chars.contains(&INVALID))
        .map(|(key, chars)| {
            let mut s = String::with_capacity(4 + 4 * chars.len());
            s.push_str(key);
            s.push(':');
            s.extend(chars.iter());
            s
        })
        .collect()
}

/// Joins the source text into one sentence, returning it with the set of
/// char indices at which a segment ends.
///
/// Line breaks are normalised the way Python's text mode reads a file, then
/// treated as separators.
pub fn normalize_input(data: &str) -> (Vec<char>, HashSet<usize>) {
    let data = data
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\u{2581}");
    let mut sentence = Vec::new();
    let mut sep_indices = HashSet::new();
    for chunk in crate::py_strip(&data).split(SEP) {
        sentence.extend(chunk.chars());
        sep_indices.insert(sentence.len());
    }
    (sentence, sep_indices)
}

/// Encodes the source text `data`, writing one line per character to `out`.
pub fn encode<W: Write>(data: &str, scale: i64, mut out: W) -> io::Result<()> {
    let (sentence, sep_indices) = normalize_input(data);
    let len = sentence.len();
    let at = |i: usize| sentence[i];
    for i in 1..=len {
        let feature = get_feature([
            if i > 2 { at(i - 3) } else { INVALID },
            if i > 1 { at(i - 2) } else { INVALID },
            at(i - 1),
            if i < len { at(i) } else { INVALID },
            if i + 1 < len { at(i + 1) } else { INVALID },
            if i + 2 < len { at(i + 2) } else { INVALID },
        ]);
        let label = if sep_indices.contains(&i) {
            scale
        } else {
            -scale
        };
        write!(out, "{label}")?;
        for f in &feature {
            write!(out, "\t{f}")?;
        }
        writeln!(out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_str(data: &str, scale: i64) -> String {
        let mut out = Vec::new();
        encode(data, scale, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    // Ported from upstream's tests/test_encode_data.py.

    #[test]
    fn get_feature_standard() {
        let feature = get_feature(['a', 'b', 'c', 'd', 'e', 'f']);
        assert_eq!(
            feature,
            [
                "UW1:a", "UW2:b", "UW3:c", "UW4:d", "UW5:e", "UW6:f", "BW1:bc", "BW2:cd", "BW3:de",
                "TW1:abc", "TW2:bcd", "TW3:cde", "TW4:def",
            ]
        );
    }

    #[test]
    fn get_feature_with_invalid() {
        let feature = get_feature(['a', 'a', INVALID, 'a', 'a', 'a']);
        assert!(!feature.iter().any(|f| f.starts_with("UW3:")));
        assert!(!feature.iter().any(|f| f.starts_with("BW2:")));
        assert_eq!(
            feature,
            [
                "UW1:a", "UW2:a", "UW4:a", "UW5:a", "UW6:a", "BW3:aa", "TW4:aaa"
            ]
        );
    }

    #[test]
    fn normalize_input_standard() {
        let (sentence, sep) = normalize_input("ABC▁DE▁FGHI");
        assert_eq!(sentence.iter().collect::<String>(), "ABCDEFGHI");
        assert_eq!(sep, HashSet::from([3, 5, 9]));
    }

    #[test]
    fn normalize_input_with_linebreaks() {
        let (sentence, sep) = normalize_input("AB\nCDE▁FG");
        assert_eq!(sentence.iter().collect::<String>(), "ABCDEFG");
        assert_eq!(sep, HashSet::from([2, 5, 7]));
    }

    #[test]
    fn normalize_input_doubled_seps() {
        let (sentence, sep) = normalize_input("ABC▁▁DE\n\nFG");
        assert_eq!(sentence.iter().collect::<String>(), "ABCDEFG");
        assert_eq!(sep, HashSet::from([3, 5, 7]));
    }

    #[test]
    fn process_with_scale() {
        // 六本木ヒルズで▁お昼を▁食べる。
        let out = encode_str("六本木ヒルズで▁お昼を▁食べる。", 16);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines.len(), 14);
        // Encoded line `k` is the boundary after char `k`, i.e. upstream's
        // `process(k + 1, ...)`.
        let negative: Vec<_> = lines[7].split('\t').collect();
        assert_eq!(negative[0], "-16");
        assert!(negative.contains(&"UW2:で"));
        let positive: Vec<_> = lines[6].split('\t').collect();
        assert_eq!(positive[0], "16");
        assert!(positive.contains(&"UW3:で"));
    }

    #[test]
    fn encode_marks_boundaries() {
        let out = encode_str("ab▁c\n", 3);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(
            lines,
            [
                "-3\tUW3:a\tUW4:b\tUW5:c\tBW2:ab\tBW3:bc\tTW3:abc",
                "3\tUW2:a\tUW3:b\tUW4:c\tBW1:ab\tBW2:bc\tTW2:abc",
                "3\tUW1:a\tUW2:b\tUW3:c\tBW1:bc\tTW1:abc",
            ]
        );
    }

    #[test]
    fn crlf_is_a_line_break() {
        assert_eq!(encode_str("ab\r\ncd", 1), encode_str("ab\ncd", 1));
    }
}
