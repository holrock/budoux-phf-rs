//! Port of `build_model.py`: aggregates a weights file into a model JSON.
//!
//! The weights file holds one score diff per training round, so a feature can
//! appear many times; its scores are summed, scaled, and truncated to an
//! integer, and features that end up at 0 are dropped. Groups and features
//! keep the order they first appear in, as upstream's JSON does.

use std::collections::HashMap;
use std::io::{self, BufRead};

/// An insertion-ordered map, standing in for a Python `dict`.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderedMap<V> {
    entries: Vec<(String, V)>,
    index: HashMap<String, usize>,
}

impl<V> Default for OrderedMap<V> {
    fn default() -> Self {
        OrderedMap {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<V> OrderedMap<V> {
    /// Python's `dict.setdefault`.
    fn entry_or(&mut self, key: &str, default: impl FnOnce() -> V) -> &mut V {
        let i = match self.index.get(key) {
            Some(&i) => i,
            None => {
                self.index.insert(key.to_string(), self.entries.len());
                self.entries.push((key.to_string(), default()));
                self.entries.len() - 1
            }
        };
        &mut self.entries[i].1
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Feature group (`UW1`, ...) → feature content → score.
pub type Model<T> = OrderedMap<OrderedMap<T>>;

/// Sums the score diffs in a weights file by feature (upstream's
/// `aggregate_scores`). A feature is split into group and content at its first
/// colon.
pub fn aggregate_scores<R: BufRead>(reader: R) -> io::Result<Model<f64>> {
    let mut model = Model::<f64>::default();
    crate::for_each_line(reader, |lineno, row| {
        let row = crate::py_strip(row);
        if row.is_empty() {
            return Ok(());
        }
        let mut cols = row.split('\t');
        let feature = cols.next().unwrap_or("");
        let score = cols
            .next()
            .ok_or_else(|| crate::invalid_data(lineno, "missing score"))?;
        let score: f64 = score
            .trim()
            .parse()
            .map_err(|e| crate::invalid_data(lineno, format!("bad score {score:?}: {e}")))?;
        let (group, content) = feature
            .split_once(':')
            .ok_or_else(|| crate::invalid_data(lineno, format!("bad feature {feature:?}")))?;
        *model
            .entry_or(group, OrderedMap::default)
            .entry_or(content, || 0.0) += score;
        Ok(())
    })?;
    Ok(model)
}

/// Scales every score by `scale` and truncates it toward zero, dropping the
/// ones that become 0 (upstream's `round_model`).
pub fn round_model(model: &Model<f64>, scale: i64) -> Model<i64> {
    let mut rounded = Model::<i64>::default();
    for (group, features) in model.iter() {
        for (content, &score) in features.iter() {
            let scaled = (score * scale as f64) as i64;
            if scaled != 0 {
                *rounded
                    .entry_or(group, OrderedMap::default)
                    .entry_or(content, || 0) = scaled;
            }
        }
    }
    rounded
}

/// Serialises the model as compact JSON, non-ASCII characters unescaped
/// (upstream's `json.dump(..., ensure_ascii=False, separators=(',', ':'))`).
pub fn to_json(model: &Model<i64>) -> String {
    let quote = |s: &str| serde_json::to_string(s).expect("a str always serialises");
    let mut out = String::from("{");
    for (i, (group, features)) in model.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&quote(group));
        out.push_str(":{");
        for (j, (content, score)) in features.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&quote(content));
            out.push(':');
            out.push_str(&score.to_string());
        }
        out.push('}');
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from upstream's tests/test_build_model.py.

    fn to_vecs<T: Clone>(model: &Model<T>) -> Vec<(String, Vec<(String, T)>)> {
        model
            .iter()
            .map(|(g, fs)| {
                (
                    g.to_string(),
                    fs.iter().map(|(c, s)| (c.to_string(), s.clone())).collect(),
                )
            })
            .collect()
    }

    fn assert_close(model: &Model<f64>, expected: &[(&str, &[(&str, f64)])]) {
        let got = to_vecs(model);
        assert_eq!(got.len(), expected.len());
        for ((g, fs), (eg, efs)) in got.iter().zip(expected) {
            assert_eq!(g, eg);
            assert_eq!(fs.len(), efs.len());
            for ((c, s), (ec, es)) in fs.iter().zip(efs.iter()) {
                assert_eq!(c, ec);
                assert!((s - es).abs() < 1e-9, "{s} != {es}");
            }
        }
    }

    #[test]
    fn aggregate_standard() {
        let weights = "AB:x\t2.893\nBC:y\t0.123\nAB:y\t2.123\nBC:y\t1.234\n";
        let model = aggregate_scores(weights.as_bytes()).unwrap();
        assert_close(
            &model,
            &[
                ("AB", &[("x", 2.893), ("y", 2.123)]),
                ("BC", &[("y", 1.357)]),
            ],
        );
    }

    #[test]
    fn aggregate_blank_line() {
        let weights = "\nAB:x\t2.893\nBC:y\t0.123\n\nAB:y\t2.123\nBC:y\t1.234\n";
        let model = aggregate_scores(weights.as_bytes()).unwrap();
        assert_close(
            &model,
            &[
                ("AB", &[("x", 2.893), ("y", 2.123)]),
                ("BC", &[("y", 1.357)]),
            ],
        );
    }

    #[test]
    fn aggregate_colon() {
        let model = aggregate_scores("AB::\t8.123".as_bytes()).unwrap();
        assert_close(&model, &[("AB", &[(":", 8.123)])]);
    }

    #[test]
    fn aggregate_bad_rows() {
        assert!(aggregate_scores("AB:x\n".as_bytes()).is_err());
        assert!(aggregate_scores("ABx\t1.0\n".as_bytes()).is_err());
    }

    #[test]
    fn round_standard() {
        let model =
            aggregate_scores("AB:x\t1.0002\nAB:y\t4.1237\nBC:z\t2.1111\n".as_bytes()).unwrap();
        let rounded = round_model(&model, 1000);
        assert_eq!(
            to_json(&rounded),
            r#"{"AB":{"x":1000,"y":4123},"BC":{"z":2111}}"#
        );
    }

    #[test]
    fn round_insignificant_score() {
        let model =
            aggregate_scores("AB:x\t0.0009\nAB:y\t4.1237\nBC:z\t2.1111\n".as_bytes()).unwrap();
        let rounded = round_model(&model, 1000);
        assert_eq!(to_json(&rounded), r#"{"AB":{"y":4123},"BC":{"z":2111}}"#);
    }

    #[test]
    fn round_truncates_toward_zero() {
        let model = aggregate_scores("A:x\t-1.9999\nB:y\t-0.0009\n".as_bytes()).unwrap();
        assert_eq!(to_json(&round_model(&model, 1000)), r#"{"A":{"x":-1999}}"#);
    }

    #[test]
    fn json_is_unescaped_utf8() {
        let model = aggregate_scores("UW1:日\t1.0\nBW2:\"\\\t2.0\n".as_bytes()).unwrap();
        assert_eq!(
            to_json(&round_model(&model, 1000)),
            r#"{"UW1":{"日":1000},"BW2":{"\"\\":2000}}"#
        );
    }
}
