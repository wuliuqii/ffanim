//! Stage 便捷层：场景创作糖层（docs/07 评审 D1/D3 落地）。
//!
//! 收敛「环境装配四件套 + 文本上屏四拍」两类样板：
//!
//! - 装配：Theme 加载、Timeline 构造、TypstEngine 与 CJK 字体注入一次到位；
//! - 上屏：`text_at`/`formula_at` 以**像素意图**（屏上字号 + 锚点落位）一步
//!   完成「排版 → 拆件 → 落位 → 注册」，`write`/`fade_in` 一行排入组动画。
//!
//! 设计原则：糖层是**纯组合器**——零新渲染能力，全部落在原语层 API 上；
//! 可随时经 [`Stage::tl`]（借用）或 [`Stage::into_timeline`]（消费）绕过，
//! 原语层语义不变，不限制上层服务化/交互产品的形态。

use ffanim_core::{ItemId, Result, Style, Timeline};
use ffanim_typst::{Anchor, FormulaOptions, TextOptions, TypstEngine};

use crate::{Anim, Color, Error, Item, RateFunc, Theme};

/// 场景工作台：主题 + 排版引擎 + 时间轴的装配上下文。
///
/// ```no_run
/// use ffanim::stage::Stage;
/// use ffanim::typst::Anchor;
///
/// let mut st = Stage::from_theme_path(
///     concat!(env!("CARGO_MANIFEST_DIR"), "/themes/default.json"),
///     960, 540, 30,
/// )?;
/// // 标题一步上屏：屏上 52px、顶边锚定 y=48、默认前景色
/// let title = st.text_at("傅里叶级数", 480.0, 48.0, 52.0, Anchor::Top)?;
/// st.fade_in(&title, 1.0);          // 整组淡入
/// st.tl().wait(0.3);                // 原语逃生舱
/// # Ok::<(), ffanim::Error>(())
/// ```
pub struct Stage {
    tl: Timeline,
    engine: TypstEngine,
    theme: Theme,
}

/// 一步上屏的组合件句柄：内部持有已注册的全部对象句柄，
/// 供 [`Stage::write`] / [`Stage::fade_in`] 整组排入动画。
///
/// 需要对组合件做原语级操作（updater、单件动画等）时，经 [`Placed::ids`]
/// 取句柄列表、经 [`Stage::tl`] 操作时间轴。
pub struct Placed {
    pub(crate) ids: Vec<ItemId>,
}

impl Placed {
    /// 组合件内已注册的句柄列表（原语级操作的逃生舱）。
    pub fn ids(&self) -> &[ItemId] {
        &self.ids
    }
}

impl Stage {
    /// 从主题 JSON 路径装配工作台（加载即完成颜色解析前置，非法值此处报错）。
    ///
    /// 内部自动完成：`Timeline::new(w, h, fps, theme.background())`、
    /// `TypstEngine` 构造与系统 CJK 字体探测注入（缺失时退回内置拉丁字体）。
    pub fn from_theme_path(
        path: impl AsRef<std::path::Path>,
        width: u32,
        height: u32,
        fps: u32,
    ) -> Result<Self> {
        Self::with_theme(Theme::from_path(path)?, width, height, fps)
    }

    /// 以已有主题装配工作台（编程构造主题的入口）。
    pub fn with_theme(theme: Theme, width: u32, height: u32, fps: u32) -> Result<Self> {
        let bg = theme.background();
        let mut engine = TypstEngine::new(16);
        if let Some((font, idx)) = ffanim_typst::find_cjk_font() {
            engine.add_font_face(&font, idx)?;
        }
        Ok(Self {
            tl: Timeline::new(width, height, fps, bg),
            engine,
            theme,
        })
    }

    /// 主题（含 `style` 默认样式与公式排版参数）。
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// 前景/文字色（主题语义色，加载时已解析）。
    pub fn foreground(&self) -> Color {
        self.theme.foreground()
    }

    /// 强调色（高亮、tracker 曲线等）。
    pub fn accent(&self) -> Color {
        self.theme.accent()
    }

    /// 对象默认样式（`Axes::to_item(&st.style())` 等）。
    pub fn style(&self) -> &Style {
        &self.theme.style
    }

    /// 文字一步上屏：屏上字号 `font_px` 像素、锚点对准 `(cx, cy)`、默认前景色。
    ///
    /// 相比原语四拍（`engine.text → fit_transform → to_items.map(with_transform)
    /// → tl.add`），这里以意图参数（px 字号 + [`Anchor`]）一次表达，无魔法数。
    pub fn text_at(
        &mut self,
        body: &str,
        cx: f64,
        cy: f64,
        font_px: f64,
        anchor: Anchor,
    ) -> Result<Placed> {
        let opts = TextOptions::new(font_px, self.theme.foreground());
        let f = self.engine.text(body, &opts)?;
        self.place(f.to_items_at(cx, cy, font_px, anchor))
    }

    /// 公式一步上屏（Typst math 语法；落位语义同 [`Stage::text_at`]）。
    pub fn formula_at(
        &mut self,
        body: &str,
        cx: f64,
        cy: f64,
        font_px: f64,
        anchor: Anchor,
    ) -> Result<Placed> {
        let opts = FormulaOptions {
            font_size_pt: font_px,
            text_color: self.theme.foreground(),
        };
        let f = self.engine.formula(body, &opts)?;
        self.place(f.to_items_at(cx, cy, font_px, anchor))
    }

    /// Write 手感：组合件逐对象 Create 错峰排入（manim `Write` 的 lag_ratio）。
    ///
    /// `lag_step = 0` 时退化为整组同窗 Create；`lag_step` 小于单段时长时
    /// 相邻对象自然交叠（描边生长逐个接力的手写感）。
    pub fn write(&mut self, h: &Placed, dur_s: f64, lag_step: f64) -> &mut Self {
        self.tl.animate_group(
            &h.ids,
            |id| Anim::create(id).during(dur_s).rate(RateFunc::Smooth),
            lag_step,
        );
        self
    }

    /// 组合件整组同窗淡入。
    pub fn fade_in(&mut self, h: &Placed, dur_s: f64) -> &mut Self {
        self.tl
            .animate_group(&h.ids, |id| Anim::fade_in(id).during(dur_s), 0.0);
        self
    }

    /// 原语逃生舱（借用）：Tracker/Updater/相机/复杂编排直接操作时间轴。
    pub fn tl(&mut self) -> &mut Timeline {
        &mut self.tl
    }

    /// 逃生舱（消费）：出片前取走时间轴（`compile()` 后 [`Stage`] 不再可用）。
    pub fn into_timeline(self) -> Timeline {
        self.tl
    }

    /// 拆件注册：对象列表逐个 add，聚为 [`Placed`] 句柄。
    fn place(&mut self, items: Vec<Item>) -> Result<Placed> {
        if items.is_empty() {
            return Err(Error::Other("排版产物为空（无可上屏对象）".into()));
        }
        let ids = items.into_iter().map(|it| self.tl.add(it)).collect();
        Ok(Placed { ids })
    }
}
