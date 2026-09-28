//! `budoux-train`: train a BudouX model from segmented text.
//!
//! ```text
//! budoux-train encode source.txt -o encoded.txt
//! budoux-train train encoded.txt -o weights.txt
//! budoux-train build weights.txt -o model.json
//! ```

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use train::{adaboost, encode, model};

const USAGE: &str = "\
Train a BudouX model (a port of BudouX's scripts/)

Usage: budoux-train <COMMAND> [OPTIONS]

Commands:
  encode  Encode segmented source text into training data (encode_data.py)
  train   Train AdaBoost on encoded data, writing the learned weights (train.py)
  build   Build a model JSON from learned weights (build_model.py)

Run `budoux-train <COMMAND> --help` for a command's options.
";

const ENCODE_USAGE: &str = "\
Encode segmented source text into training data (encode_data.py)

Usage: budoux-train encode [OPTIONS] <SOURCE_DATA>

Arguments:
  <SOURCE_DATA>        Source text, with segments separated by `▁` (U+2581) or line breaks

Options:
  -o, --outfile <PATH>  Output file for the encoded data [default: encoded_data.txt]
      --scale <N>       Weight scale for the entries [default: 1]
  -h, --help            Print help
";

const TRAIN_USAGE: &str = "\
Train AdaBoost on encoded data, writing the learned weights (train.py)

Usage: budoux-train train [OPTIONS] <ENCODED_TRAIN_DATA>

Arguments:
  <ENCODED_TRAIN_DATA>     Encoded training data

Options:
  -o, --output <PATH>      Output file for the learned weights [default: weights.txt]
      --log <PATH>         Output file for the training log [default: train.log]
      --feature-thres <N>  Minimum frequency a feature must exceed to be used [default: 10]
      --iter <N>           Number of training iterations [default: 10000]
      --out-span <N>       Iteration span to output metrics and weights [default: 100]
      --val-data <PATH>    Encoded validation data
  -h, --help               Print help
";

const BUILD_USAGE: &str = "\
Build a model JSON from learned weights (build_model.py)

Usage: budoux-train build [OPTIONS] <WEIGHT_FILE>

Arguments:
  <WEIGHT_FILE>         Learned weights

Options:
  -o, --outfile <PATH>  Output file for the model [default: model.json]
      --scale <N>       Scale factor for the output scores [default: 1000]
  -h, --help            Print help
";

enum Command {
    Encode {
        source_data: PathBuf,
        outfile: PathBuf,
        scale: i64,
    },
    Train {
        encoded_train_data: PathBuf,
        output: PathBuf,
        log: PathBuf,
        feature_thres: i64,
        iters: usize,
        out_span: usize,
        val_data: Option<PathBuf>,
    },
    Build {
        weight_file: PathBuf,
        outfile: PathBuf,
        scale: i64,
    },
}

/// Why argument parsing stopped short of a [`Command`].
#[derive(Debug, PartialEq)]
enum ArgError {
    /// `--help` was given: print this text and exit successfully.
    Help(&'static str),
    /// A usage error: print the message and the usage, and exit with 2.
    Usage(String, &'static str),
}

/// A subcommand's arguments: one positional argument, and options that each
/// take a value.
struct Args {
    usage: &'static str,
    positional: Option<OsString>,
    /// `(long name, value)` pairs; a later occurrence overrides an earlier one.
    options: Vec<(&'static str, OsString)>,
}

impl Args {
    /// Parses `args` given the options this subcommand accepts, as
    /// `(long, short)` names. Accepts `--name value`, `--name=value`,
    /// `-x value` and `--` before a positional that starts with `-`.
    fn parse(
        args: impl IntoIterator<Item = OsString>,
        known: &[(&'static str, Option<char>)],
        usage: &'static str,
    ) -> Result<Self, ArgError> {
        let err = |msg: String| ArgError::Usage(msg, usage);
        let mut parsed = Args {
            usage,
            positional: None,
            options: Vec::new(),
        };
        let mut args = args.into_iter();
        let mut only_positional = false;
        while let Some(arg) = args.next() {
            let text = arg.to_str();
            let is_option =
                !only_positional && text.is_some_and(|t| t.starts_with('-') && t != "-");
            if !is_option {
                if parsed.positional.replace(arg.clone()).is_some() {
                    return Err(err(format!("unexpected argument {arg:?}")));
                }
                continue;
            }
            let text = text.unwrap();
            if text == "--" {
                only_positional = true;
                continue;
            }
            if text == "-h" || text == "--help" {
                return Err(ArgError::Help(usage));
            }
            let (name, inline) = match text.strip_prefix("--") {
                Some(rest) => match rest.split_once('=') {
                    Some((name, value)) => (name, Some(OsString::from(value))),
                    None => (rest, None),
                },
                None => (&text[1..], None),
            };
            let long = known
                .iter()
                .find(|(long, short)| {
                    if text.starts_with("--") {
                        *long == name
                    } else {
                        short.is_some_and(|c| name == c.encode_utf8(&mut [0; 4]))
                    }
                })
                .map(|(long, _)| *long)
                .ok_or_else(|| err(format!("unexpected argument {text:?}")))?;
            let value = match inline {
                Some(v) => v,
                None => args
                    .next()
                    .ok_or_else(|| err(format!("--{long} needs a value")))?,
            };
            parsed.options.push((long, value));
        }
        Ok(parsed)
    }

    fn err(&self, msg: String) -> ArgError {
        ArgError::Usage(msg, self.usage)
    }

    fn positional(&mut self, name: &str) -> Result<PathBuf, ArgError> {
        self.positional
            .take()
            .map(PathBuf::from)
            .ok_or_else(|| self.err(format!("missing <{name}>")))
    }

    fn raw(&self, long: &str) -> Option<&OsString> {
        self.options
            .iter()
            .rev()
            .find(|(name, _)| *name == long)
            .map(|(_, v)| v)
    }

    fn path(&self, long: &str) -> Option<PathBuf> {
        self.raw(long).map(PathBuf::from)
    }

    fn path_or(&self, long: &str, default: &str) -> PathBuf {
        self.path(long).unwrap_or_else(|| default.into())
    }

    fn number<T: FromStr>(&self, long: &str, default: T) -> Result<T, ArgError>
    where
        T::Err: std::fmt::Display,
    {
        let Some(raw) = self.raw(long) else {
            return Ok(default);
        };
        raw.to_str()
            .ok_or_else(|| self.err(format!("--{long}: invalid value {raw:?}")))?
            .parse()
            .map_err(|e| self.err(format!("--{long}: invalid value {raw:?}: {e}")))
    }
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, ArgError> {
    let mut args = args.into_iter();
    let usage_err = |msg: String| ArgError::Usage(msg, USAGE);
    let command = args
        .next()
        .ok_or_else(|| usage_err("missing <COMMAND>".into()))?;
    match command.to_str() {
        Some("encode") => {
            let known = [("outfile", Some('o')), ("scale", None)];
            let mut a = Args::parse(args, &known, ENCODE_USAGE)?;
            Ok(Command::Encode {
                source_data: a.positional("SOURCE_DATA")?,
                outfile: a.path_or("outfile", "encoded_data.txt"),
                scale: a.number("scale", 1)?,
            })
        }
        Some("train") => {
            let known = [
                ("output", Some('o')),
                ("log", None),
                ("feature-thres", None),
                ("iter", None),
                ("out-span", None),
                ("val-data", None),
            ];
            let mut a = Args::parse(args, &known, TRAIN_USAGE)?;
            let out_span = a.number("out-span", 100)?;
            if out_span == 0 {
                return Err(a.err("--out-span must be at least 1".into()));
            }
            Ok(Command::Train {
                encoded_train_data: a.positional("ENCODED_TRAIN_DATA")?,
                output: a.path_or("output", "weights.txt"),
                log: a.path_or("log", "train.log"),
                feature_thres: a.number("feature-thres", 10)?,
                iters: a.number("iter", 10000)?,
                out_span,
                val_data: a.path("val-data"),
            })
        }
        Some("build") => {
            let known = [("outfile", Some('o')), ("scale", None)];
            let mut a = Args::parse(args, &known, BUILD_USAGE)?;
            Ok(Command::Build {
                weight_file: a.positional("WEIGHT_FILE")?,
                outfile: a.path_or("outfile", "model.json"),
                scale: a.number("scale", 1000)?,
            })
        }
        Some("-h" | "--help" | "help") => Err(ArgError::Help(USAGE)),
        _ => Err(usage_err(format!("unknown command {command:?}"))),
    }
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

fn run(command: Command) -> io::Result<()> {
    match command {
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
            if features.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "{}: no feature occurs more than --feature-thres ({feature_thres}) times; \
                         use more data or a lower --feature-thres",
                        encoded_train_data.display()
                    ),
                ));
            }
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
                out_span,
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
    let command = match parse_args(std::env::args_os().skip(1)) {
        Ok(command) => command,
        Err(ArgError::Help(usage)) => {
            print!("{usage}");
            return ExitCode::SUCCESS;
        }
        Err(ArgError::Usage(msg, usage)) => {
            eprintln!("error: {msg}\n\n{usage}");
            return ExitCode::from(2);
        }
    };
    match run(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, ArgError> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn train_defaults() {
        let Ok(Command::Train {
            encoded_train_data,
            output,
            log,
            feature_thres,
            iters,
            out_span,
            val_data,
        }) = parse(&["train", "encoded.txt"])
        else {
            panic!("not a train command");
        };
        assert_eq!(encoded_train_data, Path::new("encoded.txt"));
        assert_eq!(output, Path::new("weights.txt"));
        assert_eq!(log, Path::new("train.log"));
        assert_eq!((feature_thres, iters, out_span), (10, 10000, 100));
        assert_eq!(val_data, None);
    }

    #[test]
    fn train_full() {
        let args = [
            "train",
            "encoded.txt",
            "-o",
            "out.txt",
            "--log=foo.log",
            "--feature-thres",
            "100",
            "--iter",
            "10",
            "--out-span",
            "50",
            "--val-data",
            "val_encoded.txt",
        ];
        let Ok(Command::Train {
            output,
            log,
            feature_thres,
            iters,
            out_span,
            val_data,
            ..
        }) = parse(&args)
        else {
            panic!("not a train command");
        };
        assert_eq!(output, Path::new("out.txt"));
        assert_eq!(log, Path::new("foo.log"));
        assert_eq!((feature_thres, iters, out_span), (100, 10, 50));
        assert_eq!(val_data.as_deref(), Some(Path::new("val_encoded.txt")));
    }

    #[test]
    fn encode_and_build() {
        let Ok(Command::Encode {
            source_data,
            outfile,
            scale,
        }) = parse(&["encode", "--scale", "-20", "source.txt"])
        else {
            panic!("not an encode command");
        };
        assert_eq!(source_data, Path::new("source.txt"));
        assert_eq!(outfile, Path::new("encoded_data.txt"));
        assert_eq!(scale, -20);

        let Ok(Command::Build {
            weight_file,
            outfile,
            scale,
        }) = parse(&[
            "build",
            "weight.txt",
            "--outfile",
            "foo.json",
            "--scale",
            "200",
        ])
        else {
            panic!("not a build command");
        };
        assert_eq!(weight_file, Path::new("weight.txt"));
        assert_eq!(outfile, Path::new("foo.json"));
        assert_eq!(scale, 200);
    }

    #[test]
    fn positional_after_double_dash() {
        let Ok(Command::Build { weight_file, .. }) = parse(&["build", "--", "-w.txt"]) else {
            panic!("not a build command");
        };
        assert_eq!(weight_file, Path::new("-w.txt"));
    }

    #[test]
    fn help() {
        assert_eq!(parse(&["--help"]).err(), Some(ArgError::Help(USAGE)));
        assert_eq!(
            parse(&["train", "-h"]).err(),
            Some(ArgError::Help(TRAIN_USAGE))
        );
    }

    #[test]
    fn usage_errors() {
        let is_usage = |args: &[&str]| matches!(parse(args), Err(ArgError::Usage(..)));
        assert!(is_usage(&[]));
        assert!(is_usage(&["fit"]));
        assert!(is_usage(&["train"]));
        assert!(is_usage(&["train", "a.txt", "b.txt"]));
        assert!(is_usage(&["train", "a.txt", "-v"]));
        assert!(is_usage(&["train", "a.txt", "--iter"]));
        assert!(is_usage(&["train", "a.txt", "--iter", "ten"]));
        assert!(is_usage(&["train", "a.txt", "--out-span", "0"]));
        assert!(is_usage(&["encode", "a.txt", "--log", "x"]));
    }
}
