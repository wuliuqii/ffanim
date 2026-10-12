//! 文本轮廓提取：rustybuzz 整形 + ttf-parser 轮廓 → 统一贝塞尔路径。
//!
//! 文本/公式在 ffanim 中一律先提取为矢量轮廓（与 M3 公式字形共用此机制），
//! 渲染层只见路径 —— GPU/CPU 后端零特判。

use crate::{Error, Result};
use kurbo::{Affine, BezPath, PathEl, Point};
use std::path::{Path, PathBuf};
use ttf_parser::GlyphId;

/// 已加载的字体文件（单 face）。
#[derive(Debug, Clone)]
pub struct Font {
    data: Vec<u8>,
    index: u32,
}

impl Font {
    /// 从文件加载字体（.ttf / .otf）。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| Error::FontIo(e, path.to_path_buf()))?;
        Self::from_data(data)
    }

    /// 系统字体加载：显式路径优先，否则搜索常见系统位置（Linux/macOS）。
    pub fn from_system(explicit: Option<impl AsRef<Path>>) -> Result<Self> {
        match explicit {
            Some(p) => Self::from_path(p),
            None => {
                const CANDIDATES: &[&str] = &[
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/TTF/DejaVuSans.ttf",
                    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
                    "/System/Library/Fonts/Helvetica.ttc",
                    // CJK：中文标题/字幕需要（ttc 集合，face 0 含常用汉字）
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                ];
                CANDIDATES
                    .iter()
                    .map(PathBuf::from)
                    .find(|p| p.exists())
                    .ok_or(Error::NoFontFound)
                    .and_then(Self::from_path)
            }
        }
    }

    /// 从内存加载。
    pub fn from_data(data: Vec<u8>) -> Result<Self> {
        Self::validate(&data, 0)?;
        Ok(Self { data, index: 0 })
    }

    fn validate(data: &[u8], index: u32) -> Result<()> {
        ttf_parser::Face::parse(data, index)
            .map(|_| ())
            .map_err(Error::FontParse)
    }

    /// 把单行文本整形并提取为轮廓路径。
    ///
    /// 返回的路径原点在基线左侧，坐标 y 向下（屏幕方向），单位为像素。
    pub fn outline_text(&self, text: &str, font_size: f64) -> Result<BezPath> {
        let face = ttf_parser::Face::parse(&self.data, self.index).map_err(Error::FontParse)?;
        let upem = f64::from(face.units_per_em());
        // 字体坐标 y 向上 → 屏幕 y 向下：缩放并翻转
        let scale = font_size / upem;
        let to_screen = Affine::new([scale, 0.0, 0.0, -scale, 0.0, 0.0]);

        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        // rustybuzz 的 Face 是 ttf_parser::Face 的包装（附带 GSUB/GPOS 缓存）
        let hb_face = rustybuzz::Face::from_face(face);
        let output = rustybuzz::shape(&hb_face, &[], buffer);
        // 埋点：整形结果（debug，M3 Typst 公式字形共用此路径，排查缺字/对齐用）
        log::debug!(
            "文本整形: {text:?}（font_size={font_size}，glyphs={}）",
            output.glyph_infos().len()
        );

        let mut path = BezPath::new();
        let mut cursor = 0.0_f64;
        for (info, pos) in output
            .glyph_infos()
            .iter()
            .zip(output.glyph_positions().iter())
        {
            let mut glyph_path = BezPath::new();
            hb_face.outline_glyph(
                GlyphId(info.glyph_id as u16),
                &mut OutlineSink(&mut glyph_path),
            );
            // 字形平移：整形偏移 + 光标推进（字体单位 → 像素在 to_screen 中完成）
            let placement =
                Affine::translate((cursor + f64::from(pos.x_offset), f64::from(pos.y_offset)));
            path.extend(glyph_path.elements().iter().map(|el| match *el {
                PathEl::MoveTo(p) => PathEl::MoveTo(apply(to_screen, placement, p)),
                PathEl::LineTo(p) => PathEl::LineTo(apply(to_screen, placement, p)),
                PathEl::QuadTo(c, p) => PathEl::QuadTo(
                    apply(to_screen, placement, c),
                    apply(to_screen, placement, p),
                ),
                PathEl::CurveTo(c1, c2, p) => PathEl::CurveTo(
                    apply(to_screen, placement, c1),
                    apply(to_screen, placement, c2),
                    apply(to_screen, placement, p),
                ),
                other => other,
            }));
            cursor += f64::from(pos.x_advance);
        }
        Ok(path)
    }
}

/// 提取单个字形轮廓为贝塞尔路径（字体单位，y-up 坐标系，不做缩放/翻转）。
///
/// 公共 API：供 ffanim-typst 的 Typst 字形与本项目文本走同一轮廓提取实现。
pub fn outline_glyph(face: &ttf_parser::Face, gid: u16) -> Option<BezPath> {
    let mut path = BezPath::new();
    face.outline_glyph(ttf_parser::GlyphId(gid), &mut OutlineSink(&mut path))?;
    Some(path)
}

/// 先 placement（字体单位）再 to_screen（转屏幕像素）。
fn apply(to_screen: Affine, placement: Affine, p: Point) -> Point {
    to_screen * placement * p
}

/// ttf-parser 轮廓回调 → kurbo 路径。
struct OutlineSink<'a>(&'a mut BezPath);

impl ttf_parser::OutlineBuilder for OutlineSink<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(Point::new(f64::from(x), f64::from(y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(Point::new(f64::from(x), f64::from(y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(
            Point::new(f64::from(x1), f64::from(y1)),
            Point::new(f64::from(x), f64::from(y)),
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to(
            Point::new(f64::from(x1), f64::from(y1)),
            Point::new(f64::from(x2), f64::from(y2)),
            Point::new(f64::from(x), f64::from(y)),
        );
    }

    fn close(&mut self) {
        self.0.close_path();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试字体路径：CI 无字体时跳过。
    fn find_test_font() -> Option<std::path::PathBuf> {
        const CANDIDATES: &[&str] = &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ];
        CANDIDATES
            .iter()
            .map(std::path::PathBuf::from)
            .find(|p| p.exists())
    }

    #[test]
    fn 文本轮廓非空() {
        let Some(font_path) = find_test_font() else {
            eprintln!("跳过：未找到系统字体");
            return;
        };
        let font = Font::from_path(font_path).unwrap();
        let path = font.outline_text("ffanim 123", 48.0).unwrap();
        let elements: Vec<_> = path.elements().to_vec();
        assert!(!elements.is_empty(), "轮廓不应为空");
        // 所有坐标应在合理范围（48px 字号）
        for el in &elements {
            let pts: Vec<Point> = match *el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => vec![p],
                PathEl::QuadTo(c, p) => vec![c, p],
                PathEl::CurveTo(c1, c2, p) => vec![c1, c2, p],
                PathEl::ClosePath => vec![],
            };
            for p in pts {
                assert!(p.x.abs() < 1000.0 && p.y.abs() < 1000.0, "坐标越界: {p:?}");
            }
        }
    }
}
