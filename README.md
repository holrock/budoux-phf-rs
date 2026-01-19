
# budoux-phf-rs

Rust implementation of [BudouX](https://github.com/google/budoux), the machine learning-based line break organizer tool.

## Features

- **Zero runtime dictionary loading**: Uses [PHF (Perfect Hash Functions)](https://github.com/rust-phf/rust-phf) to embed dictionaries as compile-time lookup tables
- **Fast and efficient**: PHF provides O(1) lookup with minimal memory overhead
- **No external dependencies at runtime**: All data is baked into the binary
- **Multiple language support**: Japanese (ja), Simplified Chinese (zh-hans), Traditional Chinese (zh-hant), Thai (th)

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
    // Create a parser with Japanese model
    let parser = Parser::japanese_parser();

    let text = "今日は天気です。";
    let chunks: Vec<&str> = parser.parse(text);

    println!("{:?}", chunks);
    // => ["今日は", "天気です。"]
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

## Feature Flags

By default, all language models are included. You can select specific languages to reduce binary size:
```toml
[dependencies]
# Include only Japanese
budoux_phf_rs = { version = "0.1", default-features = false, features = ["ja"] }

# Include Japanese and Simplified Chinese
budoux_phf_rs = { version = "0.1", default-features = false, features = ["ja", "zh_hans"] }
```

Available features:

| Feature | Language | Description |
|---------|----------|-------------|
| `ja` | Japanese | Japanese model |
| `zh_hans` | Simplified Chinese | Simplified Chinese model |
| `zh_hant` | Traditional Chinese | Traditional Chinese model |
| `th` | Thai | Thai model |

## Build model
```shell
$ cargo run -p codegen <path/to/budoux/budoux/models> lib/src/
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- [BudouX](https://github.com/google/budoux)
- [rust-phf](https://github.com/rust-phf/rust-phf)
