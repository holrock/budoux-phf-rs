use std::collections::BTreeMap;
use std::env;
use std::fs::{File, read_dir};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

/// Model scores keyed by feature group (`UW1`, ..., `TW4`) and feature string.
///
/// `BTreeMap` rather than `HashMap`: the key order decides the order of the
/// generated `phf` entries, so this keeps the output byte-for-byte stable
/// across runs.
type Scores = BTreeMap<String, BTreeMap<String, i32>>;

const GROUPS: [&str; 13] = [
    "UW1", "UW2", "UW3", "UW4", "UW5", "UW6", "BW1", "BW2", "BW3", "TW1", "TW2", "TW3", "TW4",
];

fn gen_code(fname: &Path, output_dir: &Path) {
    let file = File::open(fname).unwrap();
    let reader = BufReader::new(file);
    let u: Scores = serde_json::from_reader(reader).unwrap();
    let prefix = fname.file_stem().unwrap().to_str().unwrap();
    write_code(&prefix.replace("-", "_"), &u, output_dir);
}

fn write_code(lang: &str, val: &Scores, output_dir: &Path) {
    let total_score: i32 = GROUPS
        .iter()
        .map(|g| val.get(*g).unwrap().values().sum::<i32>())
        .sum();
    let fname = output_dir.join(format!("model_{}.rs", lang));
    let mut out = BufWriter::new(File::create(fname).unwrap());

    write!(
        &mut out,
        r#"use super::model::*;

pub fn new() -> Model {{
    Model {{
        total_score: {},
        uw1: &UW1,
        uw2: &UW2,
        uw3: &UW3,
        uw4: &UW4,
        uw5: &UW5,
        uw6: &UW6,
        bw1: &BW1,
        bw2: &BW2,
        bw3: &BW3,
        tw1: &TW1,
        tw2: &TW2,
        tw3: &TW3,
        tw4: &TW4,
    }}
}}
"#,
        total_score
    )
    .unwrap();

    for n in GROUPS {
        write_map(&mut out, n, val.get(n).unwrap());
    }
}

fn write_map(mut out: impl Write, name: &str, val: &BTreeMap<String, i32>) {
    let mut map = phf_codegen::Map::new();
    let m = val
        .iter()
        .fold(&mut map, |acc, (k, v)| acc.entry(k, v.to_string()));
    writeln!(
        out,
        "static {}: ::phf::Map<&'static str, i16> = {};",
        name,
        m.build(),
    )
    .unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("codegen model-dir output-dir");
        return;
    }
    let model_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);
    for f in read_dir(model_dir).unwrap() {
        let f = f.unwrap().path();
        if f.extension().is_some_and(|s| s == "json") {
            gen_code(&f, output_dir);
        }
    }
}
