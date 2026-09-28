
# budoux-phf-rs

[![CI](https://github.com/holrock/budoux-phf-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/holrock/budoux-phf-rs/actions/workflows/ci.yml)

Rust implementation of [BudouX](https://github.com/google/budoux), the machine learning-based line break organizer tool.

## Features

- **Zero runtime dictionary loading**: Uses [PHF (Perfect Hash Functions)](https://github.com/rust-phf/rust-phf) to embed dictionaries as compile-time lookup tables
- **Fast and efficient**: PHF provides O(1) lookup with minimal memory overhead
- **No external dependencies at runtime**: All data is baked into the binary
- **Multiple language support**: Japanese (ja), Simplified Chinese (zh-hans), Traditional Chinese (zh-hant), Thai (th)
- **`no_std` support**: Works in `no_std` environments via `parse_with` (heap-free)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
budoux-phf-rs = "0.1"
```

## Usage

### Basic Usage

```rust
use budoux_phf_rs::Parser;

fn main() {
    let parser = Parser::japanese_parser();
    let text = "今日は天気です。";

    // Returns Vec<&str> — requires the `alloc` or `std` feature (enabled by default)
    let chunks: Vec<&str> = parser.parse(text);
    println!("{:?}", chunks);
    // => ["今日は", "天気です。"]
}
```

### `no_std` Usage

`parse_with` calls a closure for each chunk and requires no heap allocation:

```rust
use budoux_phf_rs::Parser;

fn main() {
    let parser = Parser::japanese_parser();
    let text = "今日は天気です。";

    parser.parse_with(text, |chunk| {
        // called once per chunk
        println!("{chunk}");
    });
}
```


### Other Languages

```rust
use budoux_phf_rs::Parser;

fn main() {
    // Simplified Chinese
    let parser_zh_hans = Parser::simplified_chinese_parser();

    // Traditional Chinese
    let parser_zh_hant = Parser::traditional_chinese_parser();

    // Thai
    let parser_th = Parser::thai_parser();
}
```

### Custom Model

```rust
use budoux_phf_rs::{Model, Parser, ScoreMap};

// You can use `codegen` to convert from json to a model.
// `total_score` must be the sum of every score in the maps below.
const MY_MODEL: Model = Model {
    total_score: 2552,
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
};
static UW1: ScoreMap = phf::Map { ...  };
static UW2: ScoreMap = phf::Map { ...  };
...

fn main() {
    let parser = Parser::new(MY_MODEL);
}
```


## Feature Flags

By default, the `ja`, `th`, `zh_hans` and `zh_hant` models and the `std` feature are included (`ja_knbc` is opt-in). The crate itself needs nothing from `std`; disabling the feature makes it `no_std`. You can select specific languages to reduce binary size:
```toml
[dependencies]
# Include only Japanese
budoux_phf_rs = { version = "0.1", default-features = false, features = ["ja"] }

# Include Japanese and Simplified Chinese
budoux_phf_rs = { version = "0.1", default-features = false, features = ["ja", "zh_hans"] }

# no_std with alloc (e.g. embedded with a global allocator)
budoux_phf_rs = { version = "0.1", default-features = false, features = ["alloc", "ja"] }

# no_std without alloc — only parse_with is available
budoux_phf_rs = { version = "0.1", default-features = false, features = ["ja"] }
```

Available features:

| Feature | Description |
|---------|-------------|
| `std` | Link `std` (implies `alloc`, enabled by default). Turn it off and the crate is `no_std` |
| `alloc` | Enable `parse()` returning `Vec` via the `alloc` crate |
| `ja` | Japanese model |
| `ja_knbc` | Japanese model (KNBC) |
| `zh_hans` | Simplified Chinese model |
| `zh_hant` | Traditional Chinese model |
| `th` | Thai model |

## WebAssembly

Pre-built WASM packages are available as assets on the [GitHub Releases](https://github.com/holrock/budoux-phf-rs/releases) page.

Two build targets are provided per release:

| Target | Suffix | Use case |
|--------|--------|----------|
| `web` | `-web-` | Direct use in browsers without a bundler |
| `bundler` | `-bundler-` | webpack / Vite / Rollup |

For each target, both a **full** package (all languages) and **per-language** packages are published. Per-language packages are much smaller — choose one when you only need a single language:

| File | Languages | Approx. `.wasm` size |
|------|-----------|----------------------|
| `budoux-phf-rs-wasm-<target>-vX.Y.Z.zip` | ja, th, zh-hans, zh-hant | ~276 KB |
| `budoux-phf-rs-wasm-<target>-ja-vX.Y.Z.zip` | ja | ~53 KB |
| `budoux-phf-rs-wasm-<target>-ja-knbc-vX.Y.Z.zip` | ja (KNBC) | ~49 KB |
| `budoux-phf-rs-wasm-<target>-th-vX.Y.Z.zip` | th | ~74 KB |
| `budoux-phf-rs-wasm-<target>-zh-hans-vX.Y.Z.zip` | zh-hans | ~118 KB |
| `budoux-phf-rs-wasm-<target>-zh-hant-vX.Y.Z.zip` | zh-hant | ~118 KB |

A per-language package only exports the matching `parse_*` function (e.g. the `ja` package exports only `parse_japanese`).

To build the packages yourself:

```shell
$ scripts/build-wasm.sh          # full + per-language, web + bundler
$ scripts/build-wasm.sh web      # only the web target
```

### Browser (web target)

```html
<script type="module">
  import init, { parse_japanese } from './budoux_phf_rs_wasm.js';
  await init();
  console.log(parse_japanese('今日は天気です。'));
  // => ["今日は", "天気です。"]
</script>
```

### Bundler (bundler target)

```js
import init, { parse_japanese } from './budoux_phf_rs_wasm.js';
await init();
console.log(parse_japanese('今日は天気です。'));
```

## Build model
```shell
$ cargo run -p codegen <path/to/budoux/budoux/models> lib/src/
```

`codegen` also takes a single model JSON, e.g. one you trained yourself.

## Train a custom model

The `train` crate ports BudouX's training scripts ([`scripts/`](https://github.com/google/budoux/tree/main/scripts)) to Rust, so you can train a model without Python or JAX. The subcommands mirror the upstream scripts and read and write the same file formats, so any stage can be swapped for its Python counterpart.

| Subcommand | Upstream script | Input → output |
|------------|-----------------|----------------|
| `encode` | `encode_data.py` | segmented text → encoded data |
| `train` | `train.py` | encoded data → weights (AdaBoost) |
| `build` | `build_model.py` | weights → model JSON |

The source text marks segment boundaries with `▁` (U+2581); a line break also counts as a boundary:

```text
今日は▁良い▁天気ですね。
明日も▁天気でしょう。
```

```shell
$ cargo build --release -p train
$ BT=target/release/budoux-train
$ $BT encode train.txt -o encoded.txt
$ $BT encode val.txt -o val_encoded.txt
$ $BT train encoded.txt --val-data val_encoded.txt -o weights.txt --iter 10000
$ $BT build weights.txt -o my_model.json
$ cargo run -p codegen my_model.json <output-dir>   # writes model_my_model.rs
```

The options and defaults match upstream: `--feature-thres`, `--iter`, `--out-span`, `--log` for `train`, `--scale` for `encode` and `build`. Run `budoux-train <subcommand> --help` for details. `train` writes the weights file every `--out-span` rounds, so you can stop it at any point and still build a model from what it has written. It runs on every core; set `RAYON_NUM_THREADS` to limit that.

Training computes in `f64`, and its weights and log are byte-identical to upstream's `train.py` run with `JAX_ENABLE_X64=1`. Upstream runs in `float32` by default, so a default run matches until two candidate features tie within `float32` rounding, then picks differently from that round on.

To use the model, put the generated `model_my_model.rs` in your crate next to a `model` module that re-exports the types it refers to, and pass it to `Parser::new`:

```rust
mod model {
    pub use budoux_phf_rs::{Model, ScoreMap};
}
mod model_my_model;

let parser = budoux_phf_rs::Parser::new(model_my_model::new());
```

Your crate needs `phf` as a dependency. The parser stores scores as `i16`, so every score has to fit in ±32767. `codegen` stops with an error if one doesn't; if that happens, rebuild the model with a smaller `--scale`.

## Releasing

Maintainer release process is documented in [RELEASING.md](RELEASING.md).

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- [BudouX](https://github.com/google/budoux)
- [rust-phf](https://github.com/rust-phf/rust-phf)
