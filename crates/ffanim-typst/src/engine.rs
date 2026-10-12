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
use ffanim_core::{Color, Error, Result};

// ---------------------------------------------------------------------------
// 静态资源：字体 / 标准库 / 主文件 id（进程内只建一次）
// ---------------------------------------------------------------------------

use crate::formula::{Formula, FormulaOptions, FormulaPiece};
use crate::text::TextOptions;

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

/// 进程内排版 World：无文件系统，主源为内存字符串，字体集可注入。
struct MathWorld {
    text: String,
    /// 基础字体（typst-assets）+ 用户注入字体（`add_font`）。
    fonts: Arc<Vec<Font>>,
    /// 与 `fonts` 一一对应的字体簿。
    book: Arc<LazyHash<FontBook>>,
}

impl MathWorld {
    /// 以源码与字体集构造 World。
    fn new(text: String, fonts: Arc<Vec<Font>>, book: Arc<LazyHash<FontBook>>) -> Self {
        Self { text, fonts, book }
    }
}

impl World for MathWorld {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
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
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        // 固定日期：排版结果确定性（不随墙钟变化）
        Datetime::from_ymd(2026, 1, 1)
    }
}

/// 公式/文本排版引擎。缓存键 = (模式, 源码体, 字号, 颜色, 字体集代号, 版面参数)。
pub struct TypstEngine {
    cache: lru::LruCache<CacheKey, Arc<Formula>>,
    /// 排版字体集（基础 + 注入）。
    fonts: Arc<Vec<Font>>,
    /// 与 `fonts` 一一对应的字体簿。
    book: Arc<LazyHash<FontBook>>,
    /// 字体集代号：`add_font` 后递增，进缓存键（换字体不串缓存）。
    fonts_gen: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    /// 排版模式（0 = math 公式，1 = markup 文本），防同文互串。
    mode: u8,
    body: String,
    font_size_pt: u32, // f64 不可哈希，以 0.01pt 粒度量化
    text_color: String,
    fonts_gen: u64,
    wrap_pt: u32,      // 限宽（0.01pt 量化）；公式恒 0
    leading_x100: u32, // 行距（0.01em 量化）；公式恒 0
}

impl TypstEngine {
    /// 创建引擎（capacity 为缓存公式数）。
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: lru::LruCache::new(NonZeroUsize::new(capacity.max(1)).expect("capacity > 0")),
            fonts: Arc::new(FONTS.clone()),
            book: Arc::new(FONT_BOOK.clone()),
            fonts_gen: 0,
        }
    }

    /// 加载字体文件（.ttf / .otf / .ttc，集合文件加载全部 face）到排版字体集，
    /// 返回新增 face 数。之后 `formula` / `text` 排版即可选用该字体。
    ///
    /// 缓存键含字体集代号：注入字体不会与之前的结果串缓存。
    pub fn add_font(&mut self, path: impl AsRef<std::path::Path>) -> Result<usize> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| Error::FontIo(e, path.to_path_buf()))?;
        let bytes = Bytes::new(data);
        let mut fonts = (*self.fonts).clone();
        let mut added = 0;
        let mut index = 0;
        while let Some(font) = Font::new(bytes.clone(), index) {
            fonts.push(font);
            added += 1;
            index += 1;
        }
        if added == 0 {
            return Err(Error::Other(format!("字体解析失败: {}", path.display())));
        }
        self.fonts = Arc::new(fonts);
        self.book = Arc::new(LazyHash::new(FontBook::from_fonts(self.fonts.iter())));
        self.fonts_gen += 1;
        log::debug!("字体注入: {}（+{added} face）", path.display());
        Ok(added)
    }

    /// 排版公式（Typst 数学语法，如 `x^2 + y^2 = z^2`）。
    ///
    /// 命中缓存返回同一 `Arc`（0 排版开销）；未命中则编译并写入缓存。
    pub fn formula(&mut self, body: &str, opts: &FormulaOptions) -> Result<Arc<Formula>> {
        let c = opts.text_color;
        let key = CacheKey {
            mode: 0,
            body: body.into(),
            font_size_pt: (opts.font_size_pt * 100.0).round() as u32,
            text_color: format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a),
            fonts_gen: self.fonts_gen,
            wrap_pt: 0,
            leading_x100: 0,
        };
        if let Some(hit) = self.cache.get(&key) {
            log::debug!("公式缓存命中: {body:?}");
            return Ok(Arc::clone(hit));
        }

        let t0 = std::time::Instant::now();
        let formula = Arc::new(compile_formula(body, opts, &self.fonts, &self.book)?);
        log::debug!(
            "公式排版完成: {body:?}（{:.1}ms）",
            t0.elapsed().as_secs_f64() * 1000.0
        );
        self.cache.put(key, Arc::clone(&formula));
        Ok(formula)
    }

    /// 排版富文本（Typst markup 语法：`*粗体*` `_斜体_` `` `代码` `` 均可用），
    /// 产物与公式同为 [`Formula`]（走同一条 Frame→Item 管线）。
    ///
    /// `wrap_width_pt` 限宽时自动换行（中英混排由 Typst 断行），不限宽为单行。
    pub fn text(&mut self, body: &str, opts: &TextOptions) -> Result<Arc<Formula>> {
        let c = opts.text_color;
        let key = CacheKey {
            mode: 1,
            body: body.into(),
            font_size_pt: (opts.font_size_pt * 100.0).round() as u32,
            text_color: format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a),
            fonts_gen: self.fonts_gen,
            wrap_pt: (opts.wrap_width_pt.unwrap_or(f64::INFINITY) * 100.0).round() as u32,
            leading_x100: (opts.leading_em * 100.0).round() as u32,
        };
        if let Some(hit) = self.cache.get(&key) {
            log::debug!("文本缓存命中: {body:?}");
            return Ok(Arc::clone(hit));
        }

        let t0 = std::time::Instant::now();
        let formula = Arc::new(compile_text(body, opts, &self.fonts, &self.book)?);
        log::debug!(
            "文本排版完成: {body:?}（{:.1}ms）",
            t0.elapsed().as_secs_f64() * 1000.0
        );
        self.cache.put(key, Arc::clone(&formula));
        Ok(formula)
    }
}

/// 包一层页面/样式设置后编译 math 公式，取第一页 Frame 转换为 [`Formula`]。
fn compile_formula(
    body: &str,
    opts: &FormulaOptions,
    fonts: &Arc<Vec<Font>>,
    book: &Arc<LazyHash<FontBook>>,
) -> Result<Formula> {
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
    compile_source(source, fonts, book)
}

/// 编译 markup 富文本（限宽时页面宽度 = 内容宽 + 左右 margin，恰好换行）。
fn compile_text(
    body: &str,
    opts: &TextOptions,
    fonts: &Arc<Vec<Font>>,
    book: &Arc<LazyHash<FontBook>>,
) -> Result<Formula> {
    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        opts.text_color.r, opts.text_color.g, opts.text_color.b
    );
    // margin 2pt 左右各一，页宽 = 限宽 + 4pt，正文内容宽恰为 wrap_width_pt
    let page_w = opts
        .wrap_width_pt
        .map_or_else(|| "auto".to_string(), |w| format!("{:.2}pt", w + 4.0));
    let source = format!(
        "#set page(width: {page_w}, height: auto, margin: 2pt, fill: none)\n\
         #set text(size: {}pt, fill: rgb(\"{hex}\"))\n\
         #set par(leading: {}em)\n\
         {body}",
        opts.font_size_pt, opts.leading_em
    );
    compile_source(source, fonts, book)
}

/// 编译 Typst 源码为 [`Formula`]（页宽/字号/颜色/行距由调用方拼入源码）。
fn compile_source(
    source: String,
    fonts: &Arc<Vec<Font>>,
    book: &Arc<LazyHash<FontBook>>,
) -> Result<Formula> {
    let world = MathWorld::new(source, fonts.clone(), book.clone());
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
            // 片段源文本：字形在 run 文本中的切片（供结构匹配）
            let piece_text = text.text.get(glyph.range()).map(str::to_owned);
            out.push(FormulaPiece {
                path,
                fill,
                stroke: None,
                source_range: world.range(span).map(|r| (r.start, r.end)),
                text: piece_text,
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
        text: None,
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
