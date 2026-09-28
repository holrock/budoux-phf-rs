//! Port of `train.py`: AdaBoost over the encoded data, writing weight diffs.
//!
//! Each round picks the single feature whose presence (or absence) best
//! separates boundaries from non-boundaries under the current sample weights,
//! adds its score, and reweights the samples it got wrong. The weights file
//! gets one `feature\tscore` line per round, so it is usable even if training
//! is cut short.
//!
//! This port computes in `f64`. Its weights file and log are byte-identical
//! to upstream run with `JAX_ENABLE_X64=1`; upstream's default is `float32`,
//! so against a default run the two agree until a round where two candidate
//! features are tied to within `float32` rounding, and pick differently from
//! there on. All reductions run in a fixed order, so a given input always
//! trains to the same output whatever the thread count.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use rayon::prelude::*;

/// Machine epsilon, as upstream's `EPS = float(jnp.finfo(float).eps)`.
pub const EPS: f64 = f64::EPSILON;

/// Rows per chunk for the fixed-order parallel sums.
const CHUNK: usize = 1 << 14;

/// An encoded dataset: one row per data point, holding the indices of the
/// features present in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    /// The label of each row as written in the file: its sign is the target
    /// (positive means a boundary), its magnitude the initial sample weight.
    pub y: Vec<i64>,
    /// `cols[row_ptr[r]..row_ptr[r + 1]]` are row `r`'s feature indices.
    pub row_ptr: Vec<usize>,
    pub cols: Vec<u32>,
}

impl Dataset {
    pub fn len(&self) -> usize {
        self.y.len()
    }

    pub fn is_empty(&self) -> bool {
        self.y.is_empty()
    }

    fn row(&self, r: usize) -> &[u32] {
        &self.cols[self.row_ptr[r]..self.row_ptr[r + 1]]
    }

    fn targets(&self) -> Vec<bool> {
        self.y.iter().map(|&y| y > 0).collect()
    }
}

/// The dataset transposed: for each feature, the rows it appears in.
struct Columns {
    ptr: Vec<usize>,
    rows: Vec<u32>,
}

impl Columns {
    fn new(data: &Dataset, num_features: usize) -> Self {
        let mut ptr = vec![0; num_features + 1];
        for &c in &data.cols {
            ptr[c as usize + 1] += 1;
        }
        for m in 0..num_features {
            ptr[m + 1] += ptr[m];
        }
        let mut next = ptr.clone();
        let mut rows = vec![0; data.cols.len()];
        for r in 0..data.len() {
            for &c in data.row(r) {
                rows[next[c as usize]] = r as u32;
                next[c as usize] += 1;
            }
        }
        Columns { ptr, rows }
    }

    fn col(&self, m: usize) -> &[u32] {
        &self.rows[self.ptr[m]..self.ptr[m + 1]]
    }
}

/// Splits an encoded line into its label and features, or `None` for a line
/// upstream skips (fewer than two tab-separated fields).
fn split_line(lineno: usize, line: &str) -> io::Result<Option<(i64, impl Iterator<Item = &str>)>> {
    let mut cols = crate::py_strip(line).split('\t');
    let label = cols.next().unwrap_or("");
    let mut rest = cols.peekable();
    if rest.peek().is_none() {
        return Ok(None);
    }
    let label = label
        .trim()
        .parse::<i64>()
        .map_err(|e| crate::invalid_data(lineno, format!("bad label {label:?}: {e}")))?;
    Ok(Some((label, rest)))
}

/// Lists the features whose occurrences, each counted `|label|` times, exceed
/// `thres`, most frequent first (ties in order of first appearance).
///
/// The order is the feature index order, which decides ties when training
/// picks the best feature, so it matches upstream's `Counter.most_common()`.
pub fn extract_features<R: BufRead>(reader: R, thres: i64) -> io::Result<Vec<String>> {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut counts: Vec<(String, i64)> = Vec::new();
    crate::for_each_line(reader, |lineno, line| {
        let Some((label, features)) = split_line(lineno, line)? else {
            return Ok(());
        };
        let scale = label.abs();
        for feature in features {
            match index.get(feature) {
                Some(&i) => counts[i].1 += scale,
                None => {
                    index.insert(feature.to_string(), counts.len());
                    counts.push((feature.to_string(), scale));
                }
            }
        }
        Ok(())
    })?;
    // A stable sort keeps first-appearance order among equal counts.
    counts.sort_by_key(|a| std::cmp::Reverse(a.1));
    Ok(counts
        .into_iter()
        .filter(|(_, count)| *count > thres)
        .map(|(feature, _)| feature)
        .collect())
}

/// Loads an encoded data file, keeping only the features in `findex`.
pub fn load_dataset<R: BufRead>(reader: R, findex: &HashMap<String, u32>) -> io::Result<Dataset> {
    let mut data = Dataset {
        y: Vec::new(),
        row_ptr: vec![0],
        cols: Vec::new(),
    };
    crate::for_each_line(reader, |lineno, line| {
        let Some((label, features)) = split_line(lineno, line)? else {
            return Ok(());
        };
        data.y.push(label);
        data.cols
            .extend(features.filter_map(|f| findex.get(f).copied()));
        data.row_ptr.push(data.cols.len());
        Ok(())
    })?;
    Ok(data)
}

/// Maps each feature to its index.
pub fn feature_index(features: &[String]) -> HashMap<String, u32> {
    features
        .iter()
        .enumerate()
        .map(|(i, f)| (f.clone(), i as u32))
        .collect()
}

/// Predicts each row of `data`: a boundary when the sum of the scores of the
/// features present outweighs that of those absent.
pub fn predict(scores: &[f64], data: &Dataset) -> Vec<bool> {
    let total: f64 = scores.iter().sum();
    (0..data.len())
        .into_par_iter()
        .map(|r| {
            let present: f64 = data.row(r).iter().map(|&c| scores[c as usize]).sum();
            2.0 * present - total > 0.0
        })
        .collect()
}

/// Evaluation metrics, as upstream's `Result`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub tp: usize,
    pub tn: usize,
    pub fp: usize,
    pub fn_: usize,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub fscore: f64,
}

pub fn get_metrics(pred: &[bool], actual: &[bool]) -> Metrics {
    let (mut tp, mut tn, mut fp, mut fn_) = (0, 0, 0, 0);
    for (&p, &a) in pred.iter().zip(actual) {
        match (p, a) {
            (true, true) => tp += 1,
            (false, false) => tn += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
        }
    }
    let (tpf, tnf, fpf, fnf) = (tp as f64, tn as f64, fp as f64, fn_ as f64);
    let accuracy = (tpf + tnf) / (tpf + tnf + fpf + fnf + EPS);
    let precision = tpf / (tpf + fpf + EPS);
    let recall = tpf / (tpf + fnf + EPS);
    let fscore = 2.0 * precision * recall / (precision + recall + EPS);
    Metrics {
        tp,
        tn,
        fp,
        fn_,
        accuracy,
        precision,
        recall,
        fscore,
    }
}

/// Sums `xs` in fixed-size chunks, so the result does not depend on how rayon
/// schedules the work.
fn fixed_order_sum(xs: &[f64]) -> f64 {
    xs.par_chunks(CHUNK)
        .map(|c| c.iter().sum::<f64>())
        .collect::<Vec<_>>()
        .iter()
        .sum()
}

/// The training state: the sample weights and the feature scores.
pub struct Booster {
    cols: Columns,
    y: Vec<bool>,
    /// Sample weights, summing to 1.
    pub w: Vec<f64>,
    /// Contribution score of each feature.
    pub scores: Vec<f64>,
    // Scratch buffers, kept across rounds.
    signed_w: Vec<f64>,
    res: Vec<f64>,
    in_best: Vec<bool>,
}

impl Booster {
    /// Starts training on `data`, whose feature indices are below
    /// `num_features`. The initial sample weights are the label magnitudes.
    pub fn new(data: &Dataset, num_features: usize) -> Self {
        let abs: Vec<f64> = data.y.iter().map(|&y| y.unsigned_abs() as f64).collect();
        let total = fixed_order_sum(&abs);
        let w = abs.iter().map(|a| a / total).collect();
        Self::with_weights(data, num_features, w)
    }

    fn with_weights(data: &Dataset, num_features: usize, w: Vec<f64>) -> Self {
        let n = data.len();
        Booster {
            cols: Columns::new(data, num_features),
            y: data.targets(),
            w,
            scores: vec![0.0; num_features],
            signed_w: vec![0.0; n],
            res: vec![0.0; num_features],
            in_best: vec![false; n],
        }
    }

    /// Runs one round (upstream's `update`), returning the chosen feature's
    /// index and the score added to it.
    ///
    /// # Panics
    ///
    /// If there are no features.
    pub fn update(&mut self) -> (usize, f64) {
        let (w, y) = (&self.w, &self.y);
        // res[m] = w.(Y ^ X[:, m]) = w.Y - (w * (2Y - 1)).X[:, m]: the weighted
        // error of predicting "boundary" exactly where feature m is absent.
        self.signed_w
            .par_iter_mut()
            .zip(w.par_iter().zip(y.par_iter()))
            .for_each(|(s, (&w, &y))| *s = if y { w } else { -w });
        let (cols, signed_w) = (&self.cols, &self.signed_w);
        // w.Y: the positive entries of signed_w are exactly the boundary rows.
        let wy: f64 = signed_w
            .par_chunks(CHUNK)
            .map(|c| c.iter().filter(|&&s| s > 0.0).sum::<f64>())
            .collect::<Vec<_>>()
            .iter()
            .sum();
        self.res
            .par_iter_mut()
            .enumerate()
            .with_min_len(256)
            .for_each(|(m, res)| {
                let dot: f64 = cols.col(m).iter().map(|&r| signed_w[r as usize]).sum();
                *res = wy - dot;
            });

        // The best stump has the error furthest from 0.5 in either direction:
        // err = min(res, 1 - res). The first minimum wins, as `argmin`.
        let mut best = 0;
        let mut err_min = f64::INFINITY;
        for (m, &res) in self.res.iter().enumerate() {
            let err = 0.5 - (res - 0.5).abs();
            if err < err_min {
                best = m;
                err_min = err;
            }
        }
        // `positivity`: the feature's presence signals a boundary.
        let positivity = self.res[best] < 0.5;
        let amount = ((1.0 - err_min) / (err_min + EPS)).ln();

        for &r in self.cols.col(best) {
            self.in_best[r as usize] = true;
        }
        let factor = amount.exp();
        let in_best = &self.in_best;
        self.w
            .par_iter_mut()
            .zip(y.par_iter().zip(in_best.par_iter()))
            .for_each(|(w, (&y, &x))| {
                if (y ^ x) == positivity {
                    *w *= factor;
                }
            });
        for &r in self.cols.col(best) {
            self.in_best[r as usize] = false;
        }
        let total = fixed_order_sum(&self.w);
        self.w.par_iter_mut().for_each(|w| *w /= total);

        let score = if positivity { amount } else { -amount };
        self.scores[best] += score;
        (best, score)
    }
}

/// Where [`fit`] writes its output.
pub struct Outputs<'o> {
    /// The weights file: one `feature\tscore` line per round.
    pub weights: &'o mut dyn Write,
    /// The training log: a TSV of metrics every `out_span` rounds.
    pub log: &'o mut dyn Write,
    /// Human-readable progress, as upstream prints to stdout.
    pub progress: &'o mut dyn Write,
}

fn write_metrics(progress: &mut dyn Write, name: &str, m: &Metrics) -> io::Result<()> {
    writeln!(progress, "{name} accuracy:\t{:.5}", m.accuracy)?;
    writeln!(progress, "{name} prec.:\t{:.5}", m.precision)?;
    writeln!(progress, "{name} recall:\t{:.5}", m.recall)?;
    writeln!(progress, "{name} fscore:\t{:.5}", m.fscore)?;
    writeln!(progress)
}

/// Trains for `iters` rounds (upstream's `fit`), flushing the weights and
/// metrics every `out_span` rounds and at the end. Returns the final scores.
pub fn fit(
    train: &Dataset,
    val: Option<&Dataset>,
    features: &[String],
    iters: usize,
    out_span: usize,
    out: Outputs<'_>,
) -> io::Result<Vec<f64>> {
    assert!(out_span > 0, "out_span must be positive");
    if features.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no features to train on",
        ));
    }
    write!(
        out.log,
        "iter\ttrain_accuracy\ttrain_precision\ttrain_recall\ttrain_fscore"
    )?;
    if val.is_some() {
        write!(
            out.log,
            "\ttest_accuracy\ttest_precision\ttest_recall\ttest_fscore"
        )?;
    }
    writeln!(out.log)?;
    out.log.flush()?;

    let y_train = train.targets();
    let y_val = val.map(Dataset::targets);
    let mut booster = Booster::new(train, features.len());
    let mut buffer: Vec<(usize, f64)> = Vec::new();

    let mut output_progress = |t: usize, scores: &[f64], buffer: &mut Vec<(usize, f64)>| {
        for &(m, score) in buffer.iter() {
            writeln!(out.weights, "{}\t{score:.6}", features[m])?;
        }
        out.weights.flush()?;
        buffer.clear();

        writeln!(out.progress, "=== {t} ===")?;
        writeln!(out.progress)?;
        let m = get_metrics(&predict(scores, train), &y_train);
        write_metrics(out.progress, "train", &m)?;
        write!(
            out.log,
            "{t}\t{:.5}\t{:.5}\t{:.5}\t{:.5}",
            m.accuracy, m.precision, m.recall, m.fscore
        )?;
        if let (Some(val), Some(y_val)) = (val, &y_val) {
            let m = get_metrics(&predict(scores, val), y_val);
            write_metrics(out.progress, "test", &m)?;
            write!(
                out.log,
                "\t{:.5}\t{:.5}\t{:.5}\t{:.5}",
                m.accuracy, m.precision, m.recall, m.fscore
            )?;
        }
        writeln!(out.log)?;
        out.log.flush()?;
        out.progress.flush()
    };

    for t in 0..iters {
        buffer.push(booster.update());
        if (t + 1) % out_span == 0 {
            output_progress(t + 1, &booster.scores, &mut buffer)?;
        }
    }
    if !buffer.is_empty() {
        output_progress(iters, &booster.scores, &mut buffer)?;
    }
    Ok(booster.scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from upstream's tests/test_train.py.

    /// Builds a dataset from a dense 0/1 matrix.
    fn dense(x: &[&[u8]], y: &[i64]) -> Dataset {
        let mut data = Dataset {
            y: y.to_vec(),
            row_ptr: vec![0],
            cols: Vec::new(),
        };
        for row in x {
            data.cols.extend(
                row.iter()
                    .enumerate()
                    .filter(|&(_, &v)| v == 1)
                    .map(|(c, _)| c as u32),
            );
            data.row_ptr.push(data.cols.len());
        }
        data
    }

    const ENTRIES: &str = "1\tfoo\tbar\n-1\tfoo\n1\tfoo\tbar\tbaz\n1\tbar\tfoo\n-1\tbaz\tqux\n";

    #[test]
    fn extract_features_standard() {
        let features = extract_features(ENTRIES.as_bytes(), 1).unwrap();
        assert_eq!(features, ["foo", "bar", "baz"]);
    }

    #[test]
    fn extract_features_scales_by_label() {
        let features = extract_features("-5\tqux\n1\tfoo\tbar\n1\tbar\n".as_bytes(), 1).unwrap();
        assert_eq!(features, ["qux", "bar"]);
    }

    #[test]
    fn preprocess_standard() {
        let features = extract_features(ENTRIES.as_bytes(), 0).unwrap();
        assert_eq!(features, ["foo", "bar", "baz", "qux"]);
        let features = extract_features(ENTRIES.as_bytes(), 1).unwrap();
        let findex = feature_index(&features);
        let train = load_dataset(ENTRIES.as_bytes(), &findex).unwrap();
        assert_eq!(train.y, [1, -1, 1, 1, -1]);
        let rows: Vec<usize> = (0..train.len())
            .flat_map(|r| std::iter::repeat_n(r, train.row(r).len()))
            .collect();
        assert_eq!(rows, [0, 0, 1, 2, 2, 2, 3, 3, 4]);
        assert_eq!(train.cols, [0, 1, 0, 0, 1, 2, 1, 0, 2]);

        let val_entries = "1\tbar\tbaz\n-1\txyz\n1\tabc\tqux\tfoo\n";
        let val = load_dataset(val_entries.as_bytes(), &findex).unwrap();
        assert_eq!(val.y, [1, -1, 1]);
        assert_eq!(val.row_ptr, [0, 2, 2, 3]);
        assert_eq!(val.cols, [1, 2, 0]);
    }

    #[test]
    fn skips_short_lines() {
        let findex = feature_index(&["a".to_string()]);
        let data = load_dataset("1\n\n1\ta\n-1\tb\n".as_bytes(), &findex).unwrap();
        assert_eq!(data.y, [1, -1]);
        assert_eq!(data.row_ptr, [0, 1, 1]);
    }

    #[test]
    fn bad_label_is_an_error() {
        let err = extract_features("x\ta\n".as_bytes(), 0).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn predict_standard() {
        let data = dense(
            &[&[1, 1, 0], &[1, 0, 1], &[0, 1, 0], &[0, 0, 1]],
            &[1, 1, 1, 1],
        );
        let res = predict(&[0.4, 0.2, -0.3], &data);
        let expected = [
            0.4 + 0.2 - (-0.3) > 0.0,
            0.4 - 0.2 + (-0.3) > 0.0,
            -0.4 + 0.2 - (-0.3) > 0.0,
            -0.4 - 0.2 + (-0.3) > 0.0,
        ];
        assert_eq!(res, expected);
    }

    #[test]
    fn get_metrics_standard() {
        let pred = [false, false, true, false, false];
        let actual = [true, false, true, true, true];
        let m = get_metrics(&pred, &actual);
        assert_eq!((m.tp, m.tn, m.fp, m.fn_), (1, 1, 0, 3));
        assert!((m.accuracy - 2.0 / 5.0).abs() < 1e-12);
        let (p, r) = (1.0, 1.0 / 4.0);
        assert!((m.precision - p).abs() < 1e-12);
        assert!((m.recall - r).abs() < 1e-12);
        assert!((m.fscore - 2.0 * p * r / (p + r)).abs() < 1e-12);
    }

    #[test]
    fn update_standard() {
        let x: &[&[u8]] = &[
            &[1, 0, 1, 0],
            &[0, 1, 0, 0],
            &[0, 0, 0, 0],
            &[1, 0, 0, 0],
            &[0, 1, 1, 0],
        ];
        let data = dense(x, &[1, 1, -1, -1, 1]);
        let w = vec![0.1, 0.3, 0.1, 0.1, 0.4];
        let argmax = |v: &[f64]| {
            (0..v.len())
                .reduce(|a, b| if v[b] > v[a] { b } else { a })
                .unwrap()
        };
        assert_ne!(argmax(&w), 0);
        let mut booster = Booster::with_weights(&data, 4, w);
        let (best, score) = booster.update();
        assert_eq!(argmax(&booster.w), 0);
        assert_eq!(argmax(&booster.scores), 1);
        assert_eq!(best, 1);
        assert!(score > 0.0);
        assert!((booster.w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn fit_without_features_is_an_error() {
        let data = dense(&[&[], &[]], &[1, -1]);
        let (mut weights, mut log, mut progress) = (Vec::new(), Vec::new(), Vec::new());
        let err = fit(
            &data,
            None,
            &[],
            10,
            1,
            Outputs {
                weights: &mut weights,
                log: &mut log,
                progress: &mut progress,
            },
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn fit_standard() {
        // The 2nd feature is perfectly (negatively) correlated with Y.
        // Upstream's matrix has a fourth, all-ones column with no feature
        // name, which JAX silently drops; it is left out here.
        let data = dense(
            &[&[0, 1, 1], &[1, 1, 0], &[0, 0, 1], &[1, 0, 0]],
            &[0, 0, 1, 1],
        );
        // Upstream's test uses 0/1 labels; the zero-labelled rows start at
        // weight 0, as they do there.
        let features: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
        let (iters, out_span) = (5, 2);
        let (mut weights, mut log, mut progress) = (Vec::new(), Vec::new(), Vec::new());
        let scores = fit(
            &data,
            Some(&data),
            &features,
            iters,
            out_span,
            Outputs {
                weights: &mut weights,
                log: &mut log,
                progress: &mut progress,
            },
        )
        .unwrap();

        let weights = String::from_utf8(weights).unwrap();
        let weights: Vec<Vec<&str>> = weights.lines().map(|l| l.split('\t').collect()).collect();
        assert_eq!(weights[0][0], "b");
        assert_eq!(weights.len(), iters);

        let log = String::from_utf8(log).unwrap();
        let log: Vec<Vec<&str>> = log.lines().map(|l| l.split('\t').collect()).collect();
        assert_eq!(log.len(), iters.div_ceil(out_span) + 1);
        assert!(log.iter().all(|l| l.len() == log[0].len()));
        assert_eq!(log[0].len(), 9);

        assert_eq!(scores.len(), features.len());
        let mut loaded = vec![0.0; features.len()];
        for w in &weights {
            let m = features.iter().position(|f| f == w[0]).unwrap();
            loaded[m] += w[1].parse::<f64>().unwrap();
        }
        for (a, b) in scores.iter().zip(&loaded) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }
}
