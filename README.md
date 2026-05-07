
# budoux-phf-rs

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
const MY_MODEL: Mode = Model {
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
static UW1: ScoureMap = phf::Map { ...  };
static UW2: ScoureMap = phf::Map { ...  };
...

fn main() {
    let parser = Parser { model: MY_MODEL };
}
```


## Feature Flags

By default, all language models and the `std` feature are included. You can select specific languages to reduce binary size:
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
| `std` | Enable std support (implies `alloc`, enabled by default) |
| `alloc` | Enable `parse()` returning `Vec` via the `alloc` crate |
| `ja` | Japanese model |
| `ja_knbc` | Japanese model (KNBC) |
| `zh_hans` | Simplified Chinese model |
| `zh_hant` | Traditional Chinese model |
| `th` | Thai model |

## Build model
```shell
$ cargo run -p codegen <path/to/budoux/budoux/models> lib/src/
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- [BudouX](https://github.com/google/budoux)
- [rust-phf](https://github.com/rust-phf/rust-phf)
