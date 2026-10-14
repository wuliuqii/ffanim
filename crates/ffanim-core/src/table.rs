//! 表格（manim `Table` 对应）：网格线 + 单元格底色 + 文本描边路径拼组。
//!
//! 设计取舍：不用 Typst 整块 `#table()` 排版——manim 表格的核心价值是
//! **逐格/逐行/逐列的动画句柄**（行级 lagged_start 出场、单元格高亮、
//! 圈注），故在 core 侧按 [`Font::outline_text`] 逐格排版后拼组：
//! - [`TableSpec::table`]：整表 Group（一次性注册）；
//! - [`TableSpec::rows`]：行 Group 列表（行级错峰出场，z 序同序）；
//! - [`TableSpec::cell_rects`]：格几何（行/列并集辅助 [`TableSpec::row_rect`]
//!   / [`TableSpec::col_rect`]），供高亮底板与 `Item::surrounding_rect` 圈注。
//!
//! 坐标约定：表格从原点 (0,0) 向右/向下生长，摆放由调用方
//! `with_transform(Affine::translate(...))` 完成（与标注复合件同族）。

use crate::kurbo::{Affine, BezPath, Rect, Shape as _};
use crate::style::Color;
use crate::text::Font;
use crate::{Error, Item, Result};

/// 表格样式选项（数据驱动：颜色/字号来自主题，不在本模块硬编码）。
#[derive(Debug, Clone)]
pub struct TableOptions {
    /// 单元格文字字号（px，与描边路径 1:1）。
    pub font_size: f64,
    /// 单元格文字填充色（表头同色，表头强调走 `header_fill`）。
    pub text_color: Color,
    /// 表头底色（None = 无底色）。
    pub header_fill: Option<Color>,
    /// 数据格底色（None = 无底色）。
    pub cell_fill: Option<Color>,
    /// 网格线色（None = 无网格线）。
    pub grid_color: Option<Color>,
    /// 网格线宽。
    pub grid_stroke_width: f64,
    /// 外框线（manim `include_outer_lines` 对应；默认 false = 仅内线）。
    pub include_outer_lines: bool,
    /// 单元格内边距（文本到格边，自动列宽/行高的裕量）。
    pub padding: f64,
    /// 统一行高（None = 自动：max 文本高 + 2×padding）。
    pub row_height: Option<f64>,
    /// 指定列宽（None = 自动：逐列取 max 文本宽 + 2×padding）。
    pub col_widths: Option<Vec<f64>>,
}

impl TableOptions {
    /// 基础选项（字号 + 文字色；网格/内边距走默认）。
    pub fn new(font_size: f64, text_color: Color) -> Self {
        Self {
            font_size,
            text_color,
            header_fill: None,
            cell_fill: None,
            grid_color: Some(Color::new(150, 150, 150, 255)),
            grid_stroke_width: 1.5,
            include_outer_lines: false,
            padding: 10.0,
            row_height: None,
            col_widths: None,
        }
    }
}

/// 表格拼装产物：整表 + 行组 + 格几何。
#[derive(Debug, Clone)]
pub struct TableSpec {
    /// 整表（Group：底色 → 网格线 → 文本，z 序即子项序）。
    pub table: Item,
    /// 行 Group 列表（含表头行；行级动画用，如 lagged_start 出场）。
    pub rows: Vec<Item>,
    /// `[row][col]` 格矩形（含表头行，原点坐标系，未经摆放变换）。
    pub cell_rects: Vec<Vec<Rect>>,
}

impl TableSpec {
    /// 单元格矩形。
    pub fn cell(&self, row: usize, col: usize) -> Rect {
        self.cell_rects[row][col]
    }

    /// 整行并集矩形（行高亮底板用）。
    pub fn row_rect(&self, row: usize) -> Rect {
        self.cell_rects[row]
            .iter()
            .copied()
            .reduce(|a, b| a.union(b))
            .expect("行至少一列")
    }

    /// 整列并集矩形（列高亮底板用）。
    pub fn col_rect(&self, col: usize) -> Rect {
        self.cell_rects
            .iter()
            .map(|r| r[col])
            .reduce(|a, b| a.union(b))
            .expect("列至少一行")
    }
}

/// 拼装表格（manim `Table` 对应）。
///
/// `headers` 为表头行，`rows` 为数据行；列数须与表头一致。文本经
/// [`Font::outline_text`] 描边（CJK 字体由调用方注入），格内居中。
pub fn table<S: AsRef<str>>(
    headers: &[S],
    rows_data: &[Vec<S>],
    font: &Font,
    o: &TableOptions,
) -> Result<TableSpec> {
    if headers.is_empty() {
        return Err(Error::Other("表格表头不能为空".into()));
    }
    let ncols = headers.len();
    for (i, r) in rows_data.iter().enumerate() {
        if r.len() != ncols {
            return Err(Error::Other(format!(
                "第 {i} 行列数 {} 与表头 {ncols} 不一致",
                r.len()
            )));
        }
    }

    // 逐格排版 + 度量（行序 = 表头 + 数据行）
    let mut paths: Vec<Vec<BezPath>> = Vec::new();
    let mut bboxes: Vec<Vec<Rect>> = Vec::new();
    for row in std::iter::once(headers).chain(rows_data.iter().map(Vec::as_slice)) {
        let (mut ps, mut bs) = (Vec::new(), Vec::new());
        for cell in row {
            let p = font.outline_text(cell.as_ref(), o.font_size)?;
            bs.push(p.bounding_box());
            ps.push(p);
        }
        paths.push(ps);
        bboxes.push(bs);
    }
    let nrows = paths.len();

    // 列宽：指定 > 自动（逐列 max 文本宽 + 2×padding）
    let widths: Vec<f64> = match &o.col_widths {
        Some(w) => {
            if w.len() != ncols {
                return Err(Error::Other(format!(
                    "col_widths 列数 {} 与表头 {ncols} 不一致",
                    w.len()
                )));
            }
            w.clone()
        }
        None => (0..ncols)
            .map(|c| bboxes.iter().map(|r| r[c].width()).fold(0.0_f64, f64::max) + 2.0 * o.padding)
            .collect(),
    };
    // 行高：指定 > 自动（max 文本高 + 2×padding）
    let row_h = o.row_height.unwrap_or_else(|| {
        bboxes
            .iter()
            .flatten()
            .map(|b| b.height())
            .fold(0.0_f64, f64::max)
            + 2.0 * o.padding
    });

    // 几何：原点 (0,0) 起，列/行边界前缀和
    let xs: Vec<f64> = std::iter::once(0.0)
        .chain(widths.iter().scan(0.0, |acc, &w| {
            *acc += w;
            Some(*acc)
        }))
        .collect();
    let ys: Vec<f64> = std::iter::once(0.0)
        .chain(std::iter::repeat_n(row_h, nrows).scan(0.0, |acc, h| {
            *acc += h;
            Some(*acc)
        }))
        .collect();
    let cell_rects: Vec<Vec<Rect>> = (0..nrows)
        .map(|r| {
            (0..ncols)
                .map(|c| Rect::new(xs[c], ys[r], xs[c + 1], ys[r + 1]))
                .collect()
        })
        .collect();

    // 逐行拼组：底色 → 网格线（行内竖段 + 行底横线，外框按选项）→ 文本
    let vline = |x: f64, y0: f64, y1: f64, o: &TableOptions| {
        Item::line((x, y0), (x, y1)).with_stroke(o.grid_color.expect("有网格色"))
    };
    let hline = |x0: f64, x1: f64, y: f64, o: &TableOptions| {
        Item::line((x0, y), (x1, y)).with_stroke(o.grid_color.expect("有网格色"))
    };
    let mut row_items: Vec<Item> = Vec::with_capacity(nrows);
    for r in 0..nrows {
        let mut parts: Vec<Item> = Vec::new();
        // 底色（表头行走 header_fill，数据行走 cell_fill）
        let fill = if r == 0 { o.header_fill } else { o.cell_fill };
        if let Some(f) = fill {
            for &cell in &cell_rects[r] {
                parts.push(Item::rect(cell).with_fill(f));
            }
        }
        // 网格线
        if o.grid_color.is_some() {
            // 行底横线（恒画：行间分隔线；末行兼作底外框）
            parts.push(hline(xs[0], xs[ncols], ys[r + 1], o));
            // 行顶横线仅外框模式（首行顶 = 表格外框顶）
            if o.include_outer_lines && r == 0 {
                parts.push(hline(xs[0], xs[ncols], ys[0], o));
            }
            // 行内竖线段：内线恒画，外线按选项（xs 有 ncols+1 个边界）
            for (c, &x) in xs.iter().enumerate() {
                let is_outer = c == 0 || c == ncols;
                if !is_outer || o.include_outer_lines {
                    parts.push(vline(x, ys[r], ys[r + 1], o));
                }
            }
        }
        // 文本：格内居中
        for c in 0..ncols {
            let cell = cell_rects[r][c];
            let b = bboxes[r][c];
            let dx = (cell.x0 + cell.x1) / 2.0 - (b.x0 + b.x1) / 2.0;
            let dy = (cell.y0 + cell.y1) / 2.0 - (b.y0 + b.y1) / 2.0;
            parts.push(
                Item::from_path(paths[r][c].clone())
                    .with_fill(o.text_color)
                    .with_transform(Affine::translate((dx, dy))),
            );
        }
        row_items.push(Item::group(parts));
    }

    log::info!(
        "table: {nrows} 行 × {ncols} 列（{}px 行高，列宽 {widths:?}）",
        row_h.round()
    );
    let table = Item::group(row_items.clone());
    Ok(TableSpec {
        table,
        rows: row_items,
        cell_rects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Shape;

    /// 测试字体（拉丁系统字体；无字体环境返回 None，测试打点跳过）。
    fn test_font() -> Option<Font> {
        match Font::from_system(Option::<&str>::None) {
            Ok(f) => Some(f),
            Err(_) => {
                eprintln!("跳过：未找到系统字体");
                None
            }
        }
    }

    const HEADERS: [&str; 2] = ["op", "cost"];

    fn rows() -> Vec<Vec<&'static str>> {
        vec![vec!["sum", "O(n)"], vec!["sort", "O(n log n)"]]
    }

    fn opts() -> TableOptions {
        TableOptions::new(20.0, Color::new(240, 240, 240, 255))
    }

    /// 几何拼装：3×2 格矩形行列连续，整表 bbox = 外沿并集。
    #[test]
    fn 表格几何拼装() {
        let Some(font) = test_font() else { return };
        let spec = table(&HEADERS, &rows(), &font, &opts()).unwrap();
        // 3 行（表头 + 2 数据行）× 2 列
        assert_eq!(spec.cell_rects.len(), 3);
        assert_eq!(spec.cell_rects[0].len(), 2);
        // 列边界连续：同行相邻格共边
        for row in &spec.cell_rects {
            assert!((row[0].x1 - row[1].x0).abs() < 1e-9, "相邻格应共边");
        }
        // 行边界连续：同列相邻格共边
        for r in 0..2 {
            assert!((spec.cell_rects[r][0].y1 - spec.cell_rects[r + 1][0].y0).abs() < 1e-9);
        }
        // 整表 bbox = 全部格外沿并集
        let b = spec.table.bbox();
        let outer = spec.row_rect(0).union(spec.row_rect(2));
        assert!((b.x0 - outer.x0).abs() < 1e-9 && (b.x1 - outer.x1).abs() < 1e-9);
        assert!((b.y0 - outer.y0).abs() < 1e-9 && (b.y1 - outer.y1).abs() < 1e-9);
    }

    /// 自动列宽 = 逐列 max 文本宽 + 2×padding；行高 = max 文本高 + 2×padding。
    #[test]
    fn 自动尺寸取最宽最高() {
        let Some(font) = test_font() else { return };
        let o = opts();
        let spec = table(&HEADERS, &rows(), &font, &o).unwrap();
        // 列 1 的 "O(n log n)" 显著比 "cost" 宽 → 自动宽应容纳最宽文本
        let long = font.outline_text("O(n log n)", o.font_size).unwrap();
        assert!(
            (spec.cell_rects[0][1].width() - (long.bounding_box().width() + 2.0 * o.padding)).abs()
                < 1e-9
        );
        // 全部格等高（统一行高）
        for row in &spec.cell_rects {
            for cell in row {
                assert!((cell.height() - spec.cell_rects[0][0].height()).abs() < 1e-9);
            }
        }
    }

    /// 指定列宽/行高覆盖自动值。
    #[test]
    fn 指定尺寸覆盖() {
        let Some(font) = test_font() else { return };
        let o = TableOptions {
            col_widths: Some(vec![200.0, 100.0]),
            row_height: Some(48.0),
            ..opts()
        };
        let spec = table(&HEADERS, &rows(), &font, &o).unwrap();
        assert_eq!(spec.cell_rects[0][0].width(), 200.0);
        assert_eq!(spec.cell_rects[0][1].width(), 100.0);
        assert_eq!(spec.cell_rects[1][0].height(), 48.0);
    }

    /// 文本居中：带填充的子项（文本）中心应与格中心重合。
    #[test]
    fn 文本格内居中() {
        let Some(font) = test_font() else { return };
        let spec = table(&HEADERS, &rows(), &font, &opts()).unwrap();
        for (r, row_item) in spec.rows.iter().enumerate() {
            let Shape::Group(children) = &row_item.shape else {
                panic!("行应为 Group");
            };
            // 无底色配置下：带填充的子项即文本（网格线仅描边，被滤除）
            let text_items: Vec<&Item> = children
                .iter()
                .filter(|it| it.paint.fill.is_some())
                .collect();
            for (c, child) in text_items.iter().enumerate() {
                let b = child.bbox();
                let cell = spec.cell_rects[r][c];
                assert!(
                    ((b.x0 + b.x1) / 2.0 - (cell.x0 + cell.x1) / 2.0).abs() < 1e-6
                        && ((b.y0 + b.y1) / 2.0 - (cell.y0 + cell.y1) / 2.0).abs() < 1e-6,
                    "格 ({r},{c}) 文本应居中"
                );
            }
        }
    }

    /// 网格线：默认仅内线（横线 = 每行底线；竖线 = 内边界段）；外框模式补齐。
    #[test]
    fn 网格线内外开关() {
        let Some(font) = test_font() else { return };
        let count_lines = |it: &Item| -> usize {
            let Shape::Group(children) = &it.shape else {
                panic!("应为 Group")
            };
            // 整表 Group 的直接子项是行组，需下钻一层计数描边线
            children
                .iter()
                .filter_map(|row| match &row.shape {
                    Shape::Group(cells) => Some(
                        cells
                            .iter()
                            .filter(|c| c.paint.stroke.is_some() && c.paint.fill.is_none())
                            .count(),
                    ),
                    _ => None,
                })
                .sum()
        };
        // 3 行 × 2 列，仅内线：竖线 1 边界 × 3 行 = 3，横线 3（各行底线）→ 6
        let inner = table(&HEADERS, &rows(), &font, &opts()).unwrap();
        assert_eq!(count_lines(&inner.table), 6);
        // 外框模式：+ 外竖线 2 × 3 行 = 6，+ 顶横线 1 → 13
        let outer = table(
            &HEADERS,
            &rows(),
            &font,
            &TableOptions {
                include_outer_lines: true,
                ..opts()
            },
        )
        .unwrap();
        assert_eq!(count_lines(&outer.table), 13);
    }

    /// 行/列并集：整行横贯全宽，整列纵贯全高。
    #[test]
    fn 行列并集辅助() {
        let Some(font) = test_font() else { return };
        let spec = table(&HEADERS, &rows(), &font, &opts()).unwrap();
        let row = spec.row_rect(1);
        assert_eq!(row.x0, spec.cell_rects[1][0].x0);
        assert_eq!(row.x1, spec.cell_rects[1][1].x1);
        let col = spec.col_rect(0);
        assert_eq!(col.y0, spec.cell_rects[0][0].y0);
        assert_eq!(col.y1, spec.cell_rects[2][0].y1);
    }

    /// 非法输入：空表头 / 行列数不一致 / col_widths 列数不一致，均报可读错误。
    #[test]
    fn 非法输入报错() {
        let Some(font) = test_font() else { return };
        assert!(table::<&str>(&[], &rows(), &font, &opts()).is_err());
        let ragged = vec![vec!["a"], vec!["b", "c"]];
        let e = table(&["h1", "h2"], &ragged, &font, &opts()).unwrap_err();
        assert!(e.to_string().contains("不一致"), "错误应可读: {e}");
        let e = table(
            &HEADERS,
            &rows(),
            &font,
            &TableOptions {
                col_widths: Some(vec![100.0]),
                ..opts()
            },
        )
        .unwrap_err();
        assert!(e.to_string().contains("col_widths"), "错误应可读: {e}");
    }
}
