use crate::model::Model;
#[cfg(feature = "ja")]
use crate::model_ja;
#[cfg(feature = "ja_knbc")]
use crate::model_ja_knbc;
#[cfg(feature = "th")]
use crate::model_th;
#[cfg(feature = "zh_hans")]
use crate::model_zh_hans;
#[cfg(feature = "zh_hant")]
use crate::model_zh_hant;

pub struct Parser {
    pub model: Model,
}

impl Parser {
    pub fn new(model: Model) -> Parser {
        Self { model }
    }

    #[cfg(feature = "ja")]
    pub fn japanese_parser() -> Parser {
        Parser {
            model: model_ja::new(),
        }
    }
    #[cfg(feature = "ja_knbc")]
    pub fn japanese_knbc_parser() -> Parser {
        Parser {
            model: model_ja_knbc::new(),
        }
    }
    #[cfg(feature = "zh_hans")]
    pub fn simplified_chinese_parser() -> Parser {
        Parser {
            model: model_zh_hans::new(),
        }
    }
    #[cfg(feature = "zh_hant")]
    pub fn traditional_chinese_parser() -> Parser {
        Parser {
            model: model_zh_hant::new(),
        }
    }
    #[cfg(feature = "th")]
    pub fn thai_parser() -> Parser {
        Parser {
            model: model_th::new(),
        }
    }

    /// Parse `sentence` and invoke `callback` for each chunk.
    /// Available in `no_std` environments (no heap allocation required).
    pub fn parse_with<'a, F: FnMut(&'a str)>(&self, sentence: &'a str, mut callback: F) {
        if sentence.is_empty() {
            return;
        }
        let total_score = -(self.model.total_score() / 2);

        // Count total chars — needed for boundary checks in scoring.
        let len = sentence.chars().count();

        // Ring buffer (size 8) stores byte positions of consecutive chars.
        // At step i we need indices i-3..=i+3 (span 7), so size 8 avoids
        // any collision when sliding the window forward.
        let mut ring = [0usize; 8];
        let mut ring_count = 0usize;
        let mut char_iter = sentence.char_indices();

        // Pre-load first min(8, len) char byte positions.
        while ring_count < 8 {
            if let Some((byte_pos, _)) = char_iter.next() {
                ring[ring_count] = byte_pos;
                ring_count += 1;
            } else {
                break;
            }
        }

        let mut start_byte = 0usize;

        for i in 1..len {
            // Ensure chars up to i+3 are loaded into the ring.
            while ring_count <= i + 3 && ring_count < len {
                if let Some((byte_pos, _)) = char_iter.next() {
                    ring[ring_count % 8] = byte_pos;
                    ring_count += 1;
                } else {
                    break;
                }
            }

            let ci = |j: usize| ring[j % 8];

            let mut score = total_score;
            if i > 2 {
                score += self.get_score_uw1(&sentence[ci(i - 3)..ci(i - 2)]);
            }
            if i > 1 {
                score += self.get_score_uw2(&sentence[ci(i - 2)..ci(i - 1)]);
            }
            score += self.get_score_uw3(&sentence[ci(i - 1)..ci(i)]);

            if i == len - 1 {
                score += self.get_score_uw4(&sentence[ci(i)..]);
            } else {
                score += self.get_score_uw4(&sentence[ci(i)..ci(i + 1)]);
            }
            if i < len - 1 {
                if i + 1 >= len - 1 {
                    score += self.get_score_uw5(&sentence[ci(i + 1)..]);
                } else {
                    score += self.get_score_uw5(&sentence[ci(i + 1)..ci(i + 2)]);
                }
            }
            if i < len - 2 {
                if i + 2 >= len - 1 {
                    score += self.get_score_uw6(&sentence[ci(i + 2)..]);
                } else {
                    score += self.get_score_uw6(&sentence[ci(i + 2)..ci(i + 3)]);
                }
            }

            if i > 1 {
                score += self.get_score_bw1(&sentence[ci(i - 2)..ci(i)]);
            }
            if i >= len - 1 {
                score += self.get_score_bw2(&sentence[ci(i - 1)..]);
            } else {
                score += self.get_score_bw2(&sentence[ci(i - 1)..ci(i + 1)]);
            }
            if i < len - 1 {
                if i >= len - 2 {
                    score += self.get_score_bw3(&sentence[ci(i)..]);
                } else {
                    score += self.get_score_bw3(&sentence[ci(i)..ci(i + 2)]);
                }
            }

            if i > 2 {
                score += self.get_score_tw1(&sentence[ci(i - 3)..ci(i)]);
            }
            if i > 1 && i < len - 1 {
                score += self.get_score_tw2(&sentence[ci(i - 2)..ci(i + 1)]);
            }
            if i < len - 2 {
                if i + 2 >= len - 1 {
                    score += self.get_score_tw3(&sentence[ci(i - 1)..]);
                } else {
                    score += self.get_score_tw3(&sentence[ci(i - 1)..ci(i + 2)]);
                }
            }
            if i < len - 3 {
                if i + 3 >= len - 1 {
                    score += self.get_score_tw4(&sentence[ci(i)..]);
                } else {
                    score += self.get_score_tw4(&sentence[ci(i)..ci(i + 3)]);
                }
            }

            if score > 0 {
                callback(&sentence[start_byte..ci(i)]);
                start_byte = ci(i);
            }
        }
        callback(&sentence[start_byte..]);
    }

    /// Parse `sentence` and return a `Vec` of chunk slices.
    /// Requires the `alloc` (or `std`) feature.
    #[cfg(feature = "alloc")]
    pub fn parse<'a>(&self, sentence: &'a str) -> alloc::vec::Vec<&'a str> {
        let mut chunks = alloc::vec::Vec::new();
        self.parse_with(sentence, |s| chunks.push(s));
        chunks
    }

    fn get_score_uw1(&self, s: &str) -> i32 {
        *self.model.uw1.get(s).unwrap_or(&0) as i32
    }
    fn get_score_uw2(&self, s: &str) -> i32 {
        *self.model.uw2.get(s).unwrap_or(&0) as i32
    }
    fn get_score_uw3(&self, s: &str) -> i32 {
        *self.model.uw3.get(s).unwrap_or(&0) as i32
    }
    fn get_score_uw4(&self, s: &str) -> i32 {
        *self.model.uw4.get(s).unwrap_or(&0) as i32
    }
    fn get_score_uw5(&self, s: &str) -> i32 {
        *self.model.uw5.get(s).unwrap_or(&0) as i32
    }
    fn get_score_uw6(&self, s: &str) -> i32 {
        *self.model.uw6.get(s).unwrap_or(&0) as i32
    }
    fn get_score_bw1(&self, s: &str) -> i32 {
        *self.model.bw1.get(s).unwrap_or(&0) as i32
    }
    fn get_score_bw2(&self, s: &str) -> i32 {
        *self.model.bw2.get(s).unwrap_or(&0) as i32
    }
    fn get_score_bw3(&self, s: &str) -> i32 {
        *self.model.bw3.get(s).unwrap_or(&0) as i32
    }
    fn get_score_tw1(&self, s: &str) -> i32 {
        *self.model.tw1.get(s).unwrap_or(&0) as i32
    }
    fn get_score_tw2(&self, s: &str) -> i32 {
        *self.model.tw2.get(s).unwrap_or(&0) as i32
    }
    fn get_score_tw3(&self, s: &str) -> i32 {
        *self.model.tw3.get(s).unwrap_or(&0) as i32
    }
    fn get_score_tw4(&self, s: &str) -> i32 {
        *self.model.tw4.get(s).unwrap_or(&0) as i32
    }
}

#[cfg(test)]
mod tests_parse_with {
    use super::*;

    fn check<'a>(parser: &Parser, input: &'a str, expected: &[&str]) {
        let mut idx = 0;
        parser.parse_with(input, |chunk| {
            assert_eq!(chunk, expected[idx], "chunk {idx} mismatch for {input:?}");
            idx += 1;
        });
        assert_eq!(idx, expected.len(), "chunk count mismatch for {input:?}");
    }

    #[cfg(feature = "ja")]
    #[test]
    fn test_parse_with_japanese() {
        let p = Parser::japanese_parser();
        check(&p, "今日はとても良い天気です。", &["今日は", "とても", "良い", "天気です。"]);
        check(&p, "雀の宿", &["雀の", "宿"]);
        check(&p, "", &[]);
    }

    #[cfg(feature = "zh_hans")]
    #[test]
    fn test_parse_with_zh_hans() {
        let p = Parser::simplified_chinese_parser();
        check(&p, "今天是晴天。", &["今天", "是", "晴天。"]);
    }

    #[cfg(feature = "zh_hant")]
    #[test]
    fn test_parse_with_zh_hant() {
        let p = Parser::traditional_chinese_parser();
        check(&p, "今天是晴天。", &["今天", "是", "晴天。"]);
    }

    #[cfg(feature = "th")]
    #[test]
    fn test_parse_with_thai() {
        let p = Parser::thai_parser();
        check(&p, "วันนี้อากาศดี", &["วัน", "นี้", "อากาศ", "ดี"]);
    }

    #[test]
    fn test_parse_with_custom_model() {
        use crate::model::ScoreMap;
        static F: ScoreMap = ::phf::Map {
            key: 0,
            disps: &[],
            entries: &[],
        };
        let model = crate::model::Model {
            total_score: 0,
            uw1: &F, uw2: &F, uw3: &F, uw4: &F, uw5: &F, uw6: &F,
            bw1: &F, bw2: &F, bw3: &F,
            tw1: &F, tw2: &F, tw3: &F, tw4: &F,
        };
        check(&Parser::new(model), "今日は天気です。", &["今日は天気です。"]);
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn test_parse() {
        let td = vec![
            "今日は▁とても▁良い▁天気です。",
            "abcdefg の▁使命は、▁世界中の▁情報を▁整理し、▁世界中の▁人が▁アクセスできて▁使えるように▁する▁ことです。",
            "これ以上▁利用する▁場合は▁教えてください。",
            "食器は▁そのまま▁入れて▁大丈夫です。",
            "ダウンロード▁ありがとう▁ございます。",
            "ご利用▁ありがとう▁ございました。",
            "要点を▁まとめる▁必要が▁ある。",
            "目指すのは▁あらゆる▁人に▁便利な▁ソフトウェア",
            "商品が▁まもなく▁到着します。",
            "プロジェクトが▁ようやく▁日の▁目を▁見る。",
            "明け方に▁ようやく▁目覚めると、",
            "明け方▁ようやく▁目覚めると、",
            "これは▁たまたま▁見つけた▁宝物",
            "歩いていて▁たまたま▁目に▁入った▁光景",
            "あなたの▁意図した▁とおりに▁情報を▁伝える。",
            "あの▁イーハトーヴォの▁すきとおった▁風、▁夏でも▁底に▁冷たさを▁もつ▁青い▁そら、▁うつくしい▁森で▁飾られた▁モリーオ市、▁郊外の▁ぎらぎら▁ひかる▁草の▁波。",
            "購入された▁お客様のみ▁入れます。",
            "購入された▁お客様のみ▁入場できます。",
            "パワーのみ▁有効だ",
            "小さな▁つぶや▁空気中の▁ちり",
            "光が▁どんどん▁空▁いっぱいに▁広がる",
            "太陽の▁位置が▁ちがうから",
            "太陽が▁しずむころに▁帰る",
            "多すぎると▁うまく▁いかない",
            "世界の▁子どもの▁命や▁権利",
            "「ふだん▁どおり」を▁保つ",
            "おもちゃや▁遊びに▁使える",
            "コントロールできない▁ほど▁感情移入してしまう",
            "いつも▁甘えがちに▁なる",
            "存在が▁浮かび▁上がった。",
            "雀の▁宿"
        ];
        let p = Parser::japanese_parser();
        for d in td {
            let expect: Vec<_> = d.split("▁").collect();
            let input = d.replace("▁", "");
            assert_eq!(p.parse(&input), expect);
        }
    }

    #[test]
    fn test_parser_zh_hans() {
        let parser_zh_hans = Parser::simplified_chinese_parser();
        let r = parser_zh_hans.parse("今天是晴天。");
        assert_eq!(r, vec!["今天", "是", "晴天。"]);
    }

    #[test]
    fn test_parser_zh_hant() {
        let parser_zh_hant = Parser::traditional_chinese_parser();
        let r = parser_zh_hant.parse("今天是晴天。");
        assert_eq!(r, vec!["今天", "是", "晴天。"]);
    }

    #[test]
    fn test_parser_th() {
        let parser_th = Parser::thai_parser();
        let r = parser_th.parse("วันนี้อากาศดี");
        assert_eq!(r, vec!["วัน", "นี้", "อากาศ", "ดี"]);
    }

    #[test]
    fn test_custom_model() {
        use crate::model::ScoreMap;
        static F: ScoreMap = ::phf::Map {
            key: 0,
            disps: &[],
            entries: &[],
        };
        let model = Model {
            total_score: 0,
            uw1: &F,
            uw2: &F,
            uw3: &F,
            uw4: &F,
            uw5: &F,
            uw6: &F,
            bw1: &F,
            bw2: &F,
            bw3: &F,
            tw1: &F,
            tw2: &F,
            tw3: &F,
            tw4: &F,
        };
        let parser = Parser::new(model);
        let r = parser.parse("今日は天気です。");
        assert_eq!(r, vec!["今日は天気です。"]);
    }
}
