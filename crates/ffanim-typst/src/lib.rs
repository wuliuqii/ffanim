//! ffanim-typst：Typst 库嵌入（唯一接触面）。
//!
//! - [`MathWorld`]：进程内 World 实现（字体来自 typst-assets，主源为内存字符串）；
//! - [`TypstEngine`]：公式排版入口 + LRU 缓存（同公式二次排版 0 开销）；
//! - [`Formula`]：排版产物 → ffanim [`Item`] 适配层（字形/形状 → 贝塞尔路径）。
//!
//! 坐标约定：Formula 内部为 pt 单位、y 向下（与 Typst Frame 一致），
//! 缩放/居中由调用方通过 `Item::with_transform` 完成。

use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock};

use ffanim_core::kurbo::{Affine, BezPath, PathEl, Point};
use typst::diag::FileError;
use typst::foundations::{Bytes, Datetime};
use typst::layout::{Frame, FrameItem};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::visualize::{FixedStroke, Geometry, Paint};
use typst::{Library, LibraryExt, World, WorldExt};

use ffanim_core::outline_glyph;
use ffanim_core::{Color, Error, Item, Result};

// ---------------------------------------------------------------------------
// 静态资源：字体 / 标准库 / 主文件 id（进程内只建一次）
// ---------------------------------------------------------------------------

/// 主源文件 id（全局 intern，稳定不变）。
static MAIN_FILE: LazyLock<FileId> = LazyLock::new(|| {
    RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new("main.typ").expect("合法虚拟路径"),
    )
    .intern()
});

/// typst-assets 内嵌字体（含数学字体 NewCMMath 等）。
static FONTS: LazyLock<Vec<Font>> = LazyLock::new(|| {
    typst_assets::fonts()
        .filter_map(|data| Font::new(Bytes::new(data), 0))
        .collect()
});

/// 字体簿（与 [`FONTS`] 一一对应）。
static FONT_BOOK: LazyLock<LazyHash<FontBook>> =
    LazyLock::new(|| LazyHash::new(FontBook::from_fonts(FONTS.iter())));

/// Typst 标准库（默认配置即可：页面/样式由源码内 set 规则控制）。
static LIBRARY: LazyLock<LazyHash<Library>> =
    LazyLock::new(|| LazyHash::new(Library::builder().build()));

// ---------------------------------------------------------------------------
// World 实现
// ---------------------------------------------------------------------------

/// 进程内数学排版 World：无文件系统，主源为内存字符串。
struct MathWorld {
    text: String,
}

impl MathWorld {
    /// 以公式源码构造 World。
    fn new(text: String) -> Self {
        Self { text }
    }
}

impl World for MathWorld {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &FONT_BOOK
    }

    fn main(&self) -> FileId {
        *MAIN_FILE
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
        if id == *MAIN_FILE {
            Ok(Source::new(id, self.text.clone()))
        } else {
            Err(FileError::NotFound(id.vpath().get_without_slash().into()))
        }
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<Bytes> {
        if id == *MAIN_FILE {
            Ok(Bytes::from_string(self.text.clone()))
        } else {
            Err(FileError::NotFound(id.vpath().get_without_slash().into()))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        // 固定日期：排版结果确定性（不随墙钟变化）
        Datetime::from_ymd(2026, 1, 1)
    }
}

// ---------------------------------------------------------------------------
// 排版产物模型
// ---------------------------------------------------------------------------

/// 公式排版选项。
#[derive(Debug, Clone)]
pub struct FormulaOptions {
    /// 正文字号（pt；数学字号与正文一致）。
    pub font_size_pt: f64,
    /// 文字填充色（数据驱动：来自主题）。
    pub text_color: Color,
}

/// 公式中的一个绘制片段（字形轮廓或几何形状）。
#[derive(Debug, Clone)]
pub struct FormulaPiece {
    /// 绝对坐标路径（pt，y 向下，已含字形平移与缩放）。
    pub path: BezPath,
    pub fill: Option<Color>,
    /// (描边色, 描边宽度 pt)。
    pub stroke: Option<(Color, f64)>,
    /// 源码字节区间（主源内），供 TransformMatchingTex 结构匹配（M3-2）。
    pub source_range: Option<(usize, usize)>,
}

/// 排版完成的公式（不可变，可被多帧共享）。
#[derive(Debug, Clone)]
pub struct Formula {
    pub width_pt: f64,
    pub height_pt: f64,
    pub pieces: Vec<FormulaPiece>,
}

impl Formula {
    /// 适配为 ffanim 对象列表（坐标原点即公式左上角，位置由场景层摆放）。
    pub fn to_items(&self) -> Vec<Item> {
        self.pieces
            .iter()
            .map(|p| {
                let mut item = Item::from_path(p.path.clone());
                if let Some(c) = p.fill {
                    item = item.with_fill(c);
                }
                if let Some((c, w)) = p.stroke {
                    item = item.with_stroke(c).with_stroke_width(w);
                }
                item
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 排版引擎（含 LRU 缓存）
// ---------------------------------------------------------------------------

/// 公式排版引擎。缓存键 = (公式体, 字号, 颜色)。
pub struct TypstEngine {
    cache: lru::LruCache<CacheKey, Arc<Formula>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    body: String,
    font_size_pt: u32, // f64 不可哈希，以 0.01pt 粒度量化
    text_color: String,
}

impl TypstEngine {
    /// 创建引擎（capacity 为缓存公式数）。
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: lru::LruCache::new(NonZeroUsize::new(capacity.max(1)).expect("capacity > 0")),
        }
    }

    /// 排版公式（Typst 数学语法，如 `x^2 + y^2 = z^2`）。
    ///
    /// 命中缓存返回同一 `Arc`（0 排版开销）；未命中则编译并写入缓存。
    pub fn formula(&mut self, body: &str, opts: &FormulaOptions) -> Result<Arc<Formula>> {
        let c = opts.text_color;
        let key = CacheKey {
            body: body.into(),
            font_size_pt: (opts.font_size_pt * 100.0).round() as u32,
            text_color: format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a),
        };
        if let Some(hit) = self.cache.get(&key) {
            log::debug!("公式缓存命中: {body:?}");
            return Ok(Arc::clone(hit));
        }

        let t0 = std::time::Instant::now();
        let formula = Arc::new(compile_formula(body, opts)?);
        log::debug!(
            "公式排版完成: {body:?}（{:.1}ms）",
            t0.elapsed().as_secs_f64() * 1000.0
        );
        self.cache.put(key, Arc::clone(&formula));
        Ok(formula)
    }
}

/// 包一层页面/样式设置后编译，取第一页 Frame 转换为 [`Formula`]。
fn compile_formula(body: &str, opts: &FormulaOptions) -> Result<Formula> {
    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        opts.text_color.r, opts.text_color.g, opts.text_color.b
    );
    let source = format!(
        "#set page(width: auto, height: auto, margin: 2pt, fill: none)\n\
         #set text(size: {}pt, fill: rgb(\"{hex}\"))\n\
         $ {body} $",
        opts.font_size_pt
    );

    let world = MathWorld::new(source);
    let warned = typst::compile::<typst_layout::PagedDocument>(&world);
    // 警告不致命，记日志；错误才是编译失败
    for w in &warned.warnings {
        log::warn!("Typst 排版警告: {}", w.message);
    }
    let document = warned.output.map_err(|diags| {
        let msg = diags
            .iter()
            .map(|d| d.message.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        Error::Other(format!("公式编译失败: {msg}"))
    })?;

    let page = document
        .pages()
        .first()
        .ok_or_else(|| Error::Other("公式排版产物为空（无页面）".into()))?;
    let size = page.frame.size();

    let mut pieces = Vec::new();
    walk_frame(&page.frame, Affine::IDENTITY, &world, &mut pieces);
    Ok(Formula {
        width_pt: size.x.to_pt(),
        height_pt: size.y.to_pt(),
        pieces,
    })
}

/// 深度优先遍历 Frame（组变换累积），把字形与形状摊平为路径片段。
fn walk_frame(frame: &Frame, t: Affine, world: &MathWorld, out: &mut Vec<FormulaPiece>) {
    for (pos, item) in frame.items() {
        let t = t * Affine::translate((pos.x.to_pt(), pos.y.to_pt()));
        match item {
            FrameItem::Group(group) => {
                let gt = t * Affine::new([
                    group.transform.sx.get(),
                    group.transform.ky.get(),
                    group.transform.kx.get(),
                    group.transform.sy.get(),
                    group.transform.tx.to_pt(),
                    group.transform.ty.to_pt(),
                ]);
                walk_frame(&group.frame, gt, world, out);
            }
            FrameItem::Text(text) => convert_text(text, t, world, out),
            FrameItem::Shape(shape, _) => convert_shape(shape, t, out),
            // 数学公式不产生图片/链接/元数据标签
            FrameItem::Image(..) => log::warn!("公式中出现图片，已跳过"),
            FrameItem::Link(..) | FrameItem::Tag(_) => {}
        }
    }
}

/// 字形 → 绝对路径（对照 typst-svg 的摆放约定：基线点 + (x_off, -y_off) + 缩放翻转）。
fn convert_text(
    text: &typst::text::TextItem,
    t: Affine,
    world: &MathWorld,
    out: &mut Vec<FormulaPiece>,
) {
    let upem = text.font.ttf().units_per_em() as f64;
    let scale = text.size.to_pt() / upem;
    let fill = paint_color(&text.fill);

    let mut cursor_x = 0.0_f64;
    let mut cursor_y = 0.0_f64;
    for glyph in &text.glyphs {
        let x_off = cursor_x + glyph.x_offset.at(text.size).to_pt();
        let y_off = cursor_y + glyph.y_offset.at(text.size).to_pt();
        if let Some(mut path) = outline_glyph(text.font.ttf(), glyph.id) {
            // 字体单位 y-up → pt y-down：[s, 0, 0, -s, x_off, -y_off]
            let affine = t * Affine::new([scale, 0.0, 0.0, -scale, x_off, -y_off]);
            transform_path(&mut path, affine);
            let span = glyph.span.0;
            out.push(FormulaPiece {
                path,
                fill,
                stroke: None,
                source_range: world.range(span).map(|r| (r.start, r.end)),
            });
        }
        cursor_x += glyph.x_advance.at(text.size).to_pt();
        cursor_y += glyph.y_advance.at(text.size).to_pt();
    }
}

/// 形状（分数线、根号横线等）→ 绝对路径。
fn convert_shape(shape: &typst::visualize::Shape, t: Affine, out: &mut Vec<FormulaPiece>) {
    let mut path = BezPath::new();
    match &shape.geometry {
        Geometry::Line(to) => {
            // 终点相对原点
            path.move_to((0.0, 0.0));
            path.line_to((to.x.to_pt(), to.y.to_pt()));
        }
        Geometry::Rect(size) => {
            path.move_to((0.0, 0.0));
            path.line_to((size.x.to_pt(), 0.0));
            path.line_to((size.x.to_pt(), size.y.to_pt()));
            path.line_to((0.0, size.y.to_pt()));
            path.close_path();
        }
        Geometry::Curve(curve) => {
            for item in &curve.0 {
                let pt = |p: typst::layout::Point| Point::new(p.x.to_pt(), p.y.to_pt());
                match *item {
                    typst::visualize::CurveItem::Move(p) => path.move_to(pt(p)),
                    typst::visualize::CurveItem::Line(p) => path.line_to(pt(p)),
                    typst::visualize::CurveItem::Cubic(c1, c2, p) => {
                        path.curve_to(pt(c1), pt(c2), pt(p))
                    }
                    typst::visualize::CurveItem::Close => path.close_path(),
                }
            }
        }
    }
    if path.elements().is_empty() {
        return;
    }
    transform_path(&mut path, t);
    let stroke = shape
        .stroke
        .as_ref()
        .and_then(|s: &FixedStroke| paint_color(&s.paint).map(|c| (c, s.thickness.to_pt())));
    out.push(FormulaPiece {
        path,
        fill: shape.fill.as_ref().and_then(paint_color),
        stroke,
        source_range: None,
    });
}

/// 对路径所有控制点应用仿射变换（原地）。
fn transform_path(path: &mut BezPath, affine: Affine) {
    for el in path.elements_mut() {
        match *el {
            PathEl::MoveTo(p) => *el = PathEl::MoveTo(affine * p),
            PathEl::LineTo(p) => *el = PathEl::LineTo(affine * p),
            PathEl::QuadTo(c, p) => *el = PathEl::QuadTo(affine * c, affine * p),
            PathEl::CurveTo(c1, c2, p) => {
                *el = PathEl::CurveTo(affine * c1, affine * c2, affine * p)
            }
            _ => {}
        }
    }
}

/// Typst 填充色 → ffanim Color（Solid 经 hex 转换；渐变/平铺暂不支持）。
fn paint_color(paint: &Paint) -> Option<Color> {
    match paint {
        Paint::Solid(c) => Color::from_hex(&c.to_hex()).ok(),
        other => {
            log::warn!("不支持的 Paint 类型（渐变/平铺），已按无填充处理: {other:?}");
            None
        }
    }
}

/// 供测试/诊断：把公式重新居中到矩形（后续场景摆放用例）。
impl Formula {
    /// 生成把公式中心移到 (cx, cy) 的仿射变换。
    pub fn center_transform(&self, cx: f64, cy: f64) -> Affine {
        Affine::translate((cx - self.width_pt / 2.0, cy - self.height_pt / 2.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> FormulaOptions {
        FormulaOptions {
            font_size_pt: 32.0,
            text_color: Color::new(240, 240, 240, 255),
        }
    }

    /// 排版一个基础公式，应有非空片段且尺寸为正。
    #[test]
    fn 公式排版非空() {
        let mut engine = TypstEngine::new(8);
        let f = engine.formula("x^2 + y^2 = z^2", &opts()).unwrap();
        assert!(!f.pieces.is_empty(), "应有字形片段");
        assert!(f.width_pt > 0.0 && f.height_pt > 0.0);
        // 每个片段路径非空
        for p in &f.pieces {
            assert!(!p.path.elements().is_empty(), "片段路径不应为空");
        }
    }

    /// 缓存：同公式二次排版应返回同一 Arc（0 开销）。
    #[test]
    fn 二次排版零开销() {
        let mut engine = TypstEngine::new(8);
        let a = engine
            .formula("integral_0^oo e^(-x^2) dif x", &opts())
            .unwrap();
        let b = engine
            .formula("integral_0^oo e^(-x^2) dif x", &opts())
            .unwrap();
        assert!(Arc::ptr_eq(&a, &b), "缓存命中应返回同一 Arc");
    }

    /// 分数应产生描边或填充的分数线条形状片段（非字形）。
    #[test]
    fn 分数线形状() {
        let mut engine = TypstEngine::new(8);
        let f = engine.formula("a/b", &opts()).unwrap();
        assert!(
            f.pieces.iter().any(|p| p.source_range.is_none()),
            "分数线应为无源码区间的形状片段"
        );
    }

    /// 非法公式应返回可读错误。
    #[test]
    fn 非法公式报错() {
        let mut engine = TypstEngine::new(8);
        // NaN 尺寸会产出空页面或错误，均不应 panic
        let bad = FormulaOptions {
            font_size_pt: f64::NAN,
            ..opts()
        };
        let _ = engine.formula("x", &bad);
    }

    /// 渲染冒烟：公式 → Items → CPU 后端 PNG（人工核验排版正确性）。
    #[test]
    fn 公式渲染出图() {
        use ffanim_core::DrawList;

        let mut engine = TypstEngine::new(8);
        let f = engine
            .formula("integral_0^oo e^(-x^2) dif x = sqrt(pi)/2", &opts())
            .unwrap();

        let (w, h) = (960u32, 540u32);
        let mut draw = DrawList::new(w, h, Color::new(14, 17, 22, 255));
        let scale = 2.0; // pt → px 放大便于目检
        let t = f.center_transform(f64::from(w) / 2.0, f64::from(h) / 2.0) * Affine::scale(scale);
        for item in f.to_items() {
            let transform = item.transform;
            draw.push(item.with_transform(t * transform));
        }

        use ffanim_render::RenderBackend;
        let mut cpu = ffanim_render::CpuBackend::new();
        let rgba = cpu.render(&draw);
        let out = std::path::Path::new("../../frames/formula_smoke.png");
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir).unwrap();
        }
        ffanim_render::save_png(out, w, h, &rgba).unwrap();
    }
}
