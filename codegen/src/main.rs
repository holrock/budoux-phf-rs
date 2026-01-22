use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

fn gen_code(fname: &Path, output_dir: &Path) {
    let file = File::open(fname).unwrap();
    let reader = BufReader::new(file);
    let u: HashMap<String, HashMap<String, i32>> = serde_json::from_reader(reader).unwrap();
    let prefix = fname.file_stem().unwrap().to_str().unwrap();
    write_code(&prefix.replace("-", "_"), &u, output_dir);
}

fn write_code(lang: &str, val: &HashMap<String, HashMap<String, i32>>, output_dir: &Path) {
    let total_score = val.get("UW1").unwrap().values().sum::<i32>()
        + val.get("UW2").unwrap().values().sum::<i32>()
        + val.get("UW3").unwrap().values().sum::<i32>()
        + val.get("UW4").unwrap().values().sum::<i32>()
        + val.get("UW5").unwrap().values().sum::<i32>()
        + val.get("UW6").unwrap().values().sum::<i32>()
        + val.get("BW1").unwrap().values().sum::<i32>()
        + val.get("BW2").unwrap().values().sum::<i32>()
        + val.get("BW3").unwrap().values().sum::<i32>()
        + val.get("TW1").unwrap().values().sum::<i32>()
        + val.get("TW2").unwrap().values().sum::<i32>()
        + val.get("TW3").unwrap().values().sum::<i32>()
        + val.get("TW4").unwrap().values().sum::<i32>();
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

    for n in vec![
        "UW1", "UW2", "UW3", "UW4", "UW5", "UW6", "BW1", "BW2", "BW3", "TW1", "TW2", "TW3", "TW4",
    ] {
        write_map(&mut out, n, val.get(n).unwrap());
    }
}

fn write_map(mut out: impl Write, name: &str, val: &HashMap<String, i32>) {
    let mut map = phf_codegen::Map::new();
    let m = val
        .iter()
        .fold(&mut map, |acc, (k, v)| acc.entry(k, v.to_string()));
    write!(
        out,
        "static {}: ::phf::Map<&'static str, i16> = {};\n",
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
    for n in [
        "ja.json",
        "ja_knbc.json",
        "th.json",
        "zh-hans.json",
        "zh-hant.json",
    ] {
        let f = model_dir.join(n);
        gen_code(&f, &output_dir);
    }
}
