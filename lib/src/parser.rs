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
    model: Model,
}

impl Parser {
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

    pub fn parse<'a>(&self, sentence: &'a str) -> Vec<&'a str> {
        if sentence.is_empty() {
            return vec![];
        }
        let total_score = -(self.model.total_score() / 2);
        let ci: Vec<usize> = sentence.char_indices().map(|(i, _)| i).collect();
        let len = ci.len();
        let mut start = ci[0];
        let mut chunks = vec![];

        for i in 1..len {
            let mut score = total_score;
            if i > 2 {
                score += self.get_score_uw1(&sentence[ci[i - 3]..ci[i - 2]]);
            }
            if i > 1 {
                score += self.get_score_uw2(&sentence[ci[i - 2]..ci[i - 1]]);
            }
            score += self.get_score_uw3(&sentence[ci[i - 1]..ci[i]]);

            if i == len - 1 {
                score += self.get_score_uw4(&sentence[ci[i]..]);
            } else {
                score += self.get_score_uw4(&sentence[ci[i]..ci[i + 1]]);
            }
            if i < len - 1 {
                if i + 1 >= len - 1 {
                    score += self.get_score_uw5(&sentence[ci[i + 1]..]);
                } else {
                    score += self.get_score_uw5(&sentence[ci[i + 1]..ci[i + 2]]);
                }
            }
            if i < len - 2 {
                if i + 2 >= len - 1 {
                    score += self.get_score_uw6(&sentence[ci[i + 2]..]);
                } else {
                    score += self.get_score_uw6(&sentence[ci[i + 2]..ci[i + 3]]);
                }
            }

            if i > 1 {
                score += self.get_score_bw1(&sentence[ci[i - 2]..ci[i]]);
            }
            if i >= len - 1 {
                score += self.get_score_bw2(&sentence[ci[i - 1]..]);
            } else {
                score += self.get_score_bw2(&sentence[ci[i - 1]..ci[i + 1]]);
            }
            if i < len - 1 {
                if i >= len - 2 {
                    score += self.get_score_bw3(&sentence[ci[i]..]);
                } else {
                    score += self.get_score_bw3(&sentence[ci[i]..ci[i + 2]]);
                }
            }

            if i > 2 {
                score += self.get_score_tw1(&sentence[ci[i - 3]..ci[i]]);
            }
            if i > 1 && i < len - 1 {
                score += self.get_score_tw2(&sentence[ci[i - 2]..ci[i + 1]]);
            }
            if i < len - 2 {
                if i + 2 >= len - 1 {
                    score += self.get_score_tw3(&sentence[ci[i - 1]..]);
                } else {
                    score += self.get_score_tw3(&sentence[ci[i - 1]..ci[i + 2]]);
                }
            }
            if i < len - 3 {
                if i + 3 >= len - 1 {
                    score += self.get_score_tw4(&sentence[ci[i]..]);
                } else {
                    score += self.get_score_tw4(&sentence[ci[i]..ci[i + 3]]);
                }
            }
            if score > 0 {
                chunks.push(&sentence[start..ci[i]]);
                start = ci[i];
            }
        }
        if start < ci[len - 1] {
            chunks.push(&sentence[start..]);
        }
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
mod tests {
    use super::*;

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
}
