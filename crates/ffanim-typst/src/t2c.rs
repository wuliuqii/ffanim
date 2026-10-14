//! 按词着色 t2c（manim `Text(t2c=…)` 对应）。
//!
//! 机制（注入点实证）：`compile_text` 把 body 原样拼接到全局
//! `#set text(fill: rgb("…"))` 之后，markup 中内联
//! `#text(fill: rgb("#rrggbb"))[词]` 即可覆盖局部色。构建纯函数
//! [`build_t2c_markup`] 扫描原文按对序优先、全出现、非重叠地切出着色
//! 区间（其余字符逐个转义），产物经 [`TypstEngine::text`] 走既有
//! LRU 缓存管线（同输入二次排版 0 开销）。

use std::sync::Arc;

use ffanim_core::{Color, Error, Result};

use super::engine::TypstEngine;
use super::formula::Formula;
use super::text::TextOptions;

/// Typst markup 特殊字符：正文出现即反斜杠转义（保守过转义，视觉原样）。
const MARKUP_SPECIAL: &[char] = &[
    '\\', '*', '_', '`', '#', '$', '@', '[', ']', '<', '>', '\'', '"', '~', '=', '-', '+', '/',
];

/// 逐字符转义写入（命中特殊字符集合前置 `\\`）。
fn escape_into(s: &str, out: &mut String) {
    for c in s.chars() {
        if MARKUP_SPECIAL.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
}

/// 转义正文为安全的 Typst markup（不有着色注入）。
pub fn escape_markup(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    escape_into(body, &mut out);
    out
}

/// 按词着色 markup 构建纯函数：`spans = [(词, 颜色), …]`。
///
/// 匹配语义（对齐 manim t2c）：每个词的全部出现均着色；按 spans 给定
/// 顺序优先占位（先到先得），已着色区间不再重叠分配。词未在原文出现
/// 视为拼写错误报错（数据驱动配置应尽早暴露）。
pub fn build_t2c_markup(body: &str, spans: &[(&str, Color)]) -> Result<String> {
    // 收集非重叠着色区间：byte (start, end) + 颜色
    let mut ranges: Vec<(usize, usize, Color)> = Vec::new();
    for (word, color) in spans {
        if word.is_empty() {
            return Err(Error::Other("t2c 着色词为空".into()));
        }
        if !body.contains(word) {
            return Err(Error::Other(format!("t2c 着色词未在原文出现: {word:?}")));
        }
        // 已占位区间重叠（含同词自重叠）的出现跳过——全部让位合法
        for (start, _) in body.match_indices(word) {
            let end = start + word.len();
            if ranges.iter().any(|&(s, e, _)| start < e && s < end) {
                continue;
            }
            ranges.push((start, end, *color));
        }
    }
    ranges.sort_by_key(|&(s, _, _)| s);

    // 组装：未着色段转义直通，着色段包 `#text(fill: rgb("…"))[转义片段]`
    let mut out = String::with_capacity(body.len());
    let mut pos = 0usize;
    for (start, end, color) in &ranges {
        escape_into(&body[pos..*start], &mut out);
        let hex = format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
        out.push_str("#text(fill: rgb(\"");
        out.push_str(&hex);
        out.push_str("\"))[");
        escape_into(&body[*start..*end], &mut out);
        out.push(']');
        pos = *end;
    }
    escape_into(&body[pos..], &mut out);
    Ok(out)
}

impl TypstEngine {
    /// 按词着色富文本（manim `Text(t2c=…)` 对应）。
    ///
    /// `spans` 中每个词的全部出现以指定色填充，其余文字用
    /// `opts.text_color`。产物与 [`TypstEngine::text`] 同为 [`Formula`]，
    /// `to_items` / `fit_transform` 原样复用；构造后的 markup 串即缓存键。
    pub fn text_t2c(
        &mut self,
        body: &str,
        spans: &[(&str, Color)],
        opts: &TextOptions,
    ) -> Result<Arc<Formula>> {
        let markup = build_t2c_markup(body, spans)?;
        log::info!("t2c: {} 个着色词 → markup {} 字节", spans.len(), markup.len());
        self.text(&markup, opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Color = Color::new(255, 0, 0, 255);
    const GREEN: Color = Color::new(0, 255, 0, 255);

    /// 纯函数：单 span 精确产出注入 markup。
    #[test]
    fn markup构建_单词() {
        let m = build_t2c_markup("the quick brown fox", &[("quick", RED)]).unwrap();
        assert_eq!(
            m,
            "the #text(fill: rgb(\"#ff0000\"))[quick] brown fox"
        );
    }

    /// 全出现语义：重复词全部着色。
    #[test]
    fn markup构建_重复词全着色() {
        let m = build_t2c_markup("aba ccc aba", &[("aba", RED)]).unwrap();
        assert_eq!(
            m,
            "#text(fill: rgb(\"#ff0000\"))[aba] ccc #text(fill: rgb(\"#ff0000\"))[aba]"
        );
    }

    /// 对序优先：先到的 span 占位，后到的与已着色区间重叠者让位。
    #[test]
    fn markup构建_重叠对序优先() {
        // "abcd" 先占位 → "bc" 重叠被跳过
        let m = build_t2c_markup("abcd", &[("abcd", RED), ("bc", GREEN)]).unwrap();
        assert_eq!(m, "#text(fill: rgb(\"#ff0000\"))[abcd]");
        // 反序："bc" 先占位 → "abcd" 重叠被跳过
        let m = build_t2c_markup("abcd", &[("bc", GREEN), ("abcd", RED)]).unwrap();
        assert_eq!(m, "a#text(fill: rgb(\"#00ff00\"))[bc]d");
    }

    /// 特殊字符转义：着色段内外均安全转义，视觉原样。
    #[test]
    fn markup构建_转义() {
        let m = build_t2c_markup("a*b_c", &[("b", RED)]).unwrap();
        assert_eq!(
            m,
            "a\\*#text(fill: rgb(\"#ff0000\"))[b]\\_c"
        );
        let m = escape_markup("x #y [z]");
        assert_eq!(m, "x \\#y \\[z\\]");
    }

    /// 空 span 词与未命中词均报可读错误。
    #[test]
    fn markup构建_非法span报错() {
        assert!(build_t2c_markup("abc", &[("", RED)]).is_err());
        let e = build_t2c_markup("abc", &[("zzz", RED)]).unwrap_err();
        assert!(e.to_string().contains("zzz"), "错误应含词面: {e}");
    }

    /// 引擎编译：多 span 产物至少 3 种填充色（底色 + 各 span 色）。
    #[test]
    fn t2c_多色填充() {
        let mut engine = TypstEngine::new(8);
        let o = TextOptions::new(24.0, Color::new(240, 240, 240, 255));
        let f = engine
            .text_t2c("alpha beta gamma", &[("alpha", RED), ("gamma", GREEN)], &o)
            .unwrap();
        assert!(!f.pieces.is_empty());
        let fills: std::collections::HashSet<String> = f
            .pieces
            .iter()
            .filter_map(|p| {
                p.fill.map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
            })
            .collect();
        assert!(fills.contains("#ff0000"), "应含红色 span，实际 {fills:?}");
        assert!(fills.contains("#00ff00"), "应含绿色 span，实际 {fills:?}");
        assert!(fills.contains("#f0f0f0"), "应含底色，实际 {fills:?}");
    }

    /// 缓存：同输入二次排版返回同一 Arc。
    #[test]
    fn t2c_缓存命中() {
        let mut engine = TypstEngine::new(8);
        let o = TextOptions::new(24.0, Color::new(240, 240, 240, 255));
        let a = engine.text_t2c("hello world", &[("world", RED)], &o).unwrap();
        let b = engine.text_t2c("hello world", &[("world", RED)], &o).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "缓存命中应返回同一 Arc");
    }

    /// 含全部特殊字符的正文编译不报错（转义集合实证）。
    #[test]
    fn t2c_特殊字符编译() {
        let mut engine = TypstEngine::new(8);
        let o = TextOptions::new(20.0, Color::new(240, 240, 240, 255));
        let body = "a*b_c#d$e[f]g<h>i\"j'k~l=m-n+o/p\\q@r";
        let f = engine.text_t2c(body, &[("c", RED)], &o).unwrap();
        assert!(!f.pieces.is_empty());
        // 字形文本应还原原文（转义视觉无害）
        let text: String = f.pieces.iter().filter_map(|p| p.text.as_deref()).collect();
        assert!(text.contains("a*b_c"), "转义后应还原原文字形: {text:?}");
    }

    /// 中文按词着色：需系统中文字体，缺失时跳过。
    #[test]
    fn t2c_中文着色() {
        let Some((font, idx)) = crate::text::find_cjk_font() else {
            eprintln!("跳过：未找到系统中文字体");
            return;
        };
        let mut engine = TypstEngine::new(8);
        engine.add_font_face(&font, idx).unwrap();
        let o = TextOptions::new(24.0, Color::new(240, 240, 240, 255));
        let f = engine
            .text_t2c("数学动画引擎", &[("动画", RED)], &o)
            .unwrap();
        // 红色字形片段应存在（CJK 逐字成片：收集红片段文字并集覆盖"动画"）
        let red_text: String = f
            .pieces
            .iter()
            .filter(|p| p.fill.is_some_and(|c| (c.r, c.g, c.b) == (255, 0, 0)))
            .filter_map(|p| p.text.as_deref())
            .collect();
        assert!(
            red_text.contains('动') && red_text.contains('画'),
            "红色片段应覆盖「动画」两字，实际 {red_text:?}"
        );
    }
}
