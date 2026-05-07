use wasm_bindgen::prelude::*;
use budoux_phf_rs::Parser;

#[cfg(feature = "ja")]
#[wasm_bindgen]
pub fn parse_japanese(text: &str) -> Vec<String> {
    Parser::japanese_parser()
        .parse(text)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(feature = "ja_knbc")]
#[wasm_bindgen]
pub fn parse_japanese_knbc(text: &str) -> Vec<String> {
    Parser::japanese_knbc_parser()
        .parse(text)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(feature = "zh_hans")]
#[wasm_bindgen]
pub fn parse_simplified_chinese(text: &str) -> Vec<String> {
    Parser::simplified_chinese_parser()
        .parse(text)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(feature = "zh_hant")]
#[wasm_bindgen]
pub fn parse_traditional_chinese(text: &str) -> Vec<String> {
    Parser::traditional_chinese_parser()
        .parse(text)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(feature = "th")]
#[wasm_bindgen]
pub fn parse_thai(text: &str) -> Vec<String> {
    Parser::thai_parser()
        .parse(text)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}
