//! `budoux-train`: train a BudouX model from segmented text.
//!
//! ```text
//! budoux-train encode source.txt -o encoded.txt
//! budoux-train train encoded.txt -o weights.txt
//! budoux-train build weights.txt -o model.json
//! ```

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use train::{adaboost, encode, model};

#[derive(Parser)]
#[command(about = "Train a BudouX model (a port of BudouX's scripts/)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Encode segmented source text into training data (encode_data.py).
    Encode {
        /// Source text, with segments separated by `▁` (U+2581) or line breaks.
        source_data: PathBuf,
        /// Output file for the encoded data.
        #[arg(short, long, default_value = "encoded_data.txt")]
        outfile: PathBuf,
        /// Weight scale for the entries.
        #[arg(long, default_value_t = 1)]
        scale: i64,
    },
    /// Train AdaBoost on encoded data, writing the learned weights (train.py).
    Train {
        /// Encoded training data.
        encoded_train_data: PathBuf,
        /// Output file for the learned weights.
        #[arg(short, long, default_value = "weights.txt")]
        output: PathBuf,
        /// Output file for the training log.
        #[arg(long, default_value = "train.log")]
        log: PathBuf,
        /// Minimum frequency a feature must exceed to be used.
        #[arg(long, default_value_t = 10)]
        feature_thres: i64,
        /// Number of training iterations.
        #[arg(long = "iter", default_value_t = 10000)]
        iters: usize,
        /// Iteration span to output metrics and weights.
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u64).range(1..))]
        out_span: u64,
        /// Encoded validation data.
        #[arg(long)]
        val_data: Option<PathBuf>,
    },
    /// Build a model JSON from learned weights (build_model.py).
    Build {
        /// Learned weights.
        weight_file: PathBuf,
        /// Output file for the model.
        #[arg(short, long, default_value = "model.json")]
        outfile: PathBuf,
        /// Scale factor for the output scores.
        #[arg(long, default_value_t = 1000)]
        scale: i64,
    },
}

/// Prefixes an I/O error with the file it came from.
fn with_path(path: &Path) -> impl FnOnce(io::Error) -> io::Error + '_ {
    move |e| io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}

fn open(path: &Path) -> io::Result<BufReader<File>> {
    File::open(path)
        .map(BufReader::new)
        .map_err(with_path(path))
}

fn create(path: &Path) -> io::Result<BufWriter<File>> {
    File::create(path)
        .map(BufWriter::new)
        .map_err(with_path(path))
}

fn run(cli: Cli) -> io::Result<()> {
    match cli.command {
        Command::Encode {
            source_data,
            outfile,
            scale,
        } => {
            let data = std::fs::read_to_string(&source_data).map_err(with_path(&source_data))?;
            let mut out = create(&outfile)?;
            encode::encode(&data, scale, &mut out)
                .and_then(|()| out.flush())
                .map_err(with_path(&outfile))?;
            println!("Encoded training data is out at: {}", outfile.display());
        }
        Command::Train {
            encoded_train_data,
            output,
            log,
            feature_thres,
            iters,
            out_span,
            val_data,
        } => {
            let features = adaboost::extract_features(open(&encoded_train_data)?, feature_thres)
                .map_err(with_path(&encoded_train_data))?;
            let findex = adaboost::feature_index(&features);
            let train = adaboost::load_dataset(open(&encoded_train_data)?, &findex)
                .map_err(with_path(&encoded_train_data))?;
            let val = match &val_data {
                Some(path) => {
                    Some(adaboost::load_dataset(open(path)?, &findex).map_err(with_path(path))?)
                }
                None => None,
            };
            drop(findex);
            eprintln!(
                "{} features, {} training rows{}",
                features.len(),
                train.len(),
                val.as_ref()
                    .map(|v| format!(", {} validation rows", v.len()))
                    .unwrap_or_default()
            );

            let mut weights = create(&output)?;
            let mut log_out = create(&log)?;
            println!("Outputting learned weights to {} ...", output.display());
            adaboost::fit(
                &train,
                val.as_ref(),
                &features,
                iters,
                out_span as usize,
                adaboost::Outputs {
                    weights: &mut weights,
                    log: &mut log_out,
                    progress: &mut io::stdout().lock(),
                },
            )?;
            println!(
                "Training done. Export the model by passing {} to `budoux-train build`",
                output.display()
            );
        }
        Command::Build {
            weight_file,
            outfile,
            scale,
        } => {
            let model =
                model::aggregate_scores(open(&weight_file)?).map_err(with_path(&weight_file))?;
            let json = model::to_json(&model::round_model(&model, scale));
            std::fs::write(&outfile, json).map_err(with_path(&outfile))?;
            println!("Model file is exported as {}", outfile.display());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
