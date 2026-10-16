//! 3D 投影模拟（docs/07 §1.2）：球坐标相机 + 3D 图元表 + 每帧纯函数重投影。
//!
//! 路线 = manim 同款投影模拟：世界系 **z-up**（数学示教惯例），每帧
//! [`project`] 把 3D 图元变换到屏幕像素并按画家算法排序，产物即普通 2D
//! [`Item`]——vello 渲染管线零改动，描边生长/fade/clip 等现有能力免费复用。
//! 相机可动画性由字段挂 tracker 天然获得（`Anim::set_value`），渲染期纯查表，
//! 与两阶段时间轴模型一致。
//!
//! 坐标链：世界系(z-up) → [`Camera3D::view_matrix`] → 视空间(Y-up，朝 −z) →
//! [`Camera3D::proj_matrix`] → 裁剪 → 手动除 w（顺带丢弃相机身后图元）→
//! NDC → 像素（y 翻转）→ 画家算法（质心深度，远→近）。
//!
//! 坑：透视链路必须手动除 w——`transform_point3` 仿射专用返回 clip 空间
//! z（2.2 约定测试沉淀），而 `project_point3` 不回传 w、无法判相机身后，
//! 故用 `transform_point4` 一并拿到。

use glam::{DMat4, DVec3, DVec4};
use kurbo::{BezPath, PathEl, Point};

use crate::math::{look_at, perspective};
use crate::{Color, Item};

/// 3D 球坐标相机（manim 同款语义）：纯数据，可动画性由字段挂 tracker 获得。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    /// 极角：与 +z 轴夹角（0 = 正俯视，π/2 = 平视）。manim 同款。
    pub phi: f64,
    /// 方位角：xy 平面内绕 z 轴（0 = 相机在 +x 侧）。manim 同款。
    pub theta: f64,
    /// 相机到注视点距离。
    pub radius: f64,
    /// 注视点。
    pub target: DVec3,
    /// 垂直视场角（弧度）。
    pub fov: f64,
    /// 近裁剪距离（仅影响 proj 深度映射，一般用默认值）。
    pub near: f64,
    /// 远裁剪距离。
    pub far: f64,
}

impl Default for Camera3D {
    /// manim 教学场景常用起步机位：phi=75°、theta=30°、radius=10、fov=45°。
    fn default() -> Self {
        Self {
            phi: 75f64.to_radians(),
            theta: 30f64.to_radians(),
            radius: 10.0,
            target: DVec3::ZERO,
            fov: 45f64.to_radians(),
            near: 0.1,
            far: 100.0,
        }
    }
}

impl Camera3D {
    /// 相机世界坐标（球坐标 → 直角坐标）。
    pub fn eye(&self) -> DVec3 {
        let (sp, cp) = self.phi.sin_cos();
        let (st, ct) = self.theta.sin_cos();
        self.target
            + DVec3::new(
                self.radius * sp * ct,
                self.radius * sp * st,
                self.radius * cp,
            )
    }

    /// 视图矩阵：世界系(z-up) → 视空间（Y-up、相机朝 −z；glam look_at 约定）。
    ///
    /// phi→0/π（视线与 z 轴共线）时 z-up 退化，up 改取 xy 平面内垂直向量
    /// 保持连续（φ→0⁺ 的连续极限即视线水平反向）。
    pub fn view_matrix(&self) -> DMat4 {
        let (st, ct) = self.theta.sin_cos();
        let (sp, _) = self.phi.sin_cos();
        let up = if sp.abs() < 1e-8 {
            DVec3::new(-ct, -st, 0.0)
        } else {
            DVec3::Z
        };
        look_at(self.eye(), self.target, up)
    }

    /// 透视投影矩阵（OpenGL 深度 [-1,1]，2.2 锁定约定；aspect = 宽/高）。
    pub fn proj_matrix(&self, aspect: f64) -> DMat4 {
        perspective(self.fov, aspect, self.near, self.far)
    }
}

/// 图元样式（投影后 2D Item 的绘制参数；颜色由调用方给，不取自主题）。
#[derive(Debug, Clone, PartialEq)]
pub struct PrimStyle {
    /// 描边色。
    pub stroke: Color,
    /// 描边宽（像素）。
    pub stroke_width: f64,
    /// 填充色；None = 不填充。
    pub fill: Option<Color>,
}

impl PrimStyle {
    /// 纯描边样式（折线用）。
    pub fn line(color: Color, width: f64) -> Self {
        Self {
            stroke: color,
            stroke_width: width,
            fill: None,
        }
    }

    /// 填充 + 描边样式（面用；width=0 则纯填充）。
    pub fn face(fill: Color, stroke: Color, width: f64) -> Self {
        Self {
            stroke,
            stroke_width: width,
            fill: Some(fill),
        }
    }
}

/// 3D 图元（构建期生成的最小绘制单元；投影产物为对应 2D Item）。
#[derive(Debug, Clone, PartialEq)]
pub enum Prim3D {
    /// 折线（开放）：投影后为描边路径 Item。
    Polyline {
        /// 顶点序列。
        pts: Vec<DVec3>,
        /// 样式。
        style: PrimStyle,
    },
    /// 多边形（闭合填充面）：画家算法以质心深度排序。
    Polygon {
        /// 顶点序列。
        pts: Vec<DVec3>,
        /// 样式。
        style: PrimStyle,
    },
}

/// 折线图元（螺旋线/相轨迹等空间曲线直接收点）。
pub fn polyline3(pts: impl IntoIterator<Item = DVec3>, style: PrimStyle) -> Prim3D {
    Prim3D::Polyline {
        pts: pts.into_iter().collect(),
        style,
    }
}

/// 多边形面图元。
pub fn polygon3(pts: impl IntoIterator<Item = DVec3>, style: PrimStyle) -> Prim3D {
    Prim3D::Polygon {
        pts: pts.into_iter().collect(),
        style,
    }
}

/// 三维轴系构造件选项（数据驱动：几何与颜色全部显式传入，零硬编码）。
#[derive(Debug, Clone, PartialEq)]
pub struct Axes3DOpts {
    /// 三轴长度（x/y/z 各自，从原点画到 `(size.x,0,0)` 等）。
    pub size: DVec3,
    /// z=0 方格网格：坐标 `-n..=n`（步长 1）；0 = 无网格。
    pub grid: i32,
    /// 轴线色。
    pub axis: Color,
    /// 轴线宽（像素）。
    pub axis_width: f64,
    /// 网格线色。
    pub grid_line: Color,
    /// 网格线宽（像素）。
    pub grid_width: f64,
}

/// 三维轴系 + z=0 方格网格（manim `ThreeDAxes` 对应件；轴名留 2D 标注由调用方拼装）。
///
/// 构建期一次生成图元表；深度感主要靠网格，网格线比轴线细（manim 同款观感）。
pub fn axes3d(opts: &Axes3DOpts) -> Vec<Prim3D> {
    let mut prims = Vec::new();
    let axis = |end: DVec3| {
        polyline3(
            [DVec3::ZERO, end],
            PrimStyle::line(opts.axis, opts.axis_width),
        )
    };
    prims.push(axis(DVec3::new(opts.size.x, 0.0, 0.0)));
    prims.push(axis(DVec3::new(0.0, opts.size.y, 0.0)));
    prims.push(axis(DVec3::new(0.0, 0.0, opts.size.z)));
    let n = opts.grid as f64;
    if opts.grid > 0 {
        for i in -opts.grid..=opts.grid {
            let c = i as f64;
            prims.push(polyline3(
                [DVec3::new(c, -n, 0.0), DVec3::new(c, n, 0.0)],
                PrimStyle::line(opts.grid_line, opts.grid_width),
            ));
            prims.push(polyline3(
                [DVec3::new(-n, c, 0.0), DVec3::new(n, c, 0.0)],
                PrimStyle::line(opts.grid_line, opts.grid_width),
            ));
        }
    }
    prims
}

/// 参数化曲面样式（manim `Surface` 对应件；光照非目标，明暗感由棋盘双色承担）。
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyle {
    /// 棋盘双色（`(i+j)%2` 取 a/b）；None = 单色 [`SurfaceStyle::fill`]。
    pub checker: Option<(Color, Color)>,
    /// 单色模式的填充色。
    pub fill: Color,
    /// 填充不透明度 0..1（乘到 alpha；manim `fill_opacity` 同义）。
    pub opacity: f64,
    /// 面片描边色；None 或宽 0 = 不描边。
    pub stroke: Option<Color>,
    /// 面片描边宽（像素）。
    pub stroke_width: f64,
}

impl SurfaceStyle {
    /// 棋盘双色面（不透明）。
    pub fn checkerboard(a: Color, b: Color, opacity: f64) -> Self {
        Self {
            checker: Some((a, b)),
            fill: a,
            opacity,
            stroke: None,
            stroke_width: 0.0,
        }
    }

    /// 单色面（不透明）。
    pub fn solid(color: Color, opacity: f64) -> Self {
        Self {
            checker: None,
            fill: color,
            opacity,
            stroke: None,
            stroke_width: 0.0,
        }
    }
}

/// 参数化曲面网格面：`f(u,v)` 采样 → `res` 分辨率四边形面片表（棋盘配色），
/// 深度排序交由 [`project`] 画家算法承担。
///
/// manim `Surface(f, u_range, v_range, resolution, checkerboard_colors)` 对应件。
/// 顶点先采样缓存（`(nu+1)×(nv+1)` 次）再拼面片，避免每面片重复求值 4 次。
pub fn parametric_surface(
    f: impl Fn(f64, f64) -> DVec3,
    u_range: (f64, f64),
    v_range: (f64, f64),
    resolution: (u32, u32),
    style: &SurfaceStyle,
) -> Vec<Prim3D> {
    let nu = resolution.0.max(1) as usize;
    let nv = resolution.1.max(1) as usize;
    let alpha = (style.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    // 顶点表：行主序 (j, i)，j∈[0,nv] 对应 v、i∈[0,nu] 对应 u
    let mut pts = Vec::with_capacity((nu + 1) * (nv + 1));
    for j in 0..=nv {
        let v = v_range.0 + (v_range.1 - v_range.0) * j as f64 / nv as f64;
        for i in 0..=nu {
            let u = u_range.0 + (u_range.1 - u_range.0) * i as f64 / nu as f64;
            pts.push(f(u, v));
        }
    }
    let at = |i: usize, j: usize| pts[j * (nu + 1) + i];
    let mut prims = Vec::with_capacity(nu * nv);
    for j in 0..nv {
        for i in 0..nu {
            let base = match style.checker {
                Some((a, b)) => {
                    if (i + j) % 2 == 0 {
                        a
                    } else {
                        b
                    }
                }
                None => style.fill,
            };
            let fill = base.with_alpha(alpha);
            let stroke_w = if style.stroke.is_some() {
                style.stroke_width
            } else {
                0.0
            };
            prims.push(polygon3(
                [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                PrimStyle {
                    stroke: style.stroke.unwrap_or(fill),
                    stroke_width: stroke_w,
                    fill: Some(fill),
                },
            ));
        }
    }
    prims
}

/// 每帧纯函数重投影：3D 图元表 → 已按画家算法排序（远→近）的 2D Item 序列。
///
/// 产物为屏幕像素坐标（y 向下）的普通 [`Item`]，直接 add 进 Timeline 即可；
/// 配合 tracker/updater 每帧重建即得相机动画（用法见 docs/07 §1.2 骨架）。
/// 任一点位于相机身后（w ≤ 0）的图元整条丢弃——教学场景相机在场景外围，
/// 不做视锥面剪裁（manim 同款简化）。
pub fn project(prims: &[Prim3D], cam: &Camera3D, width: f64, height: f64) -> Vec<Item> {
    let vp = cam.proj_matrix(width / height) * cam.view_matrix();
    // (质心 NDC z, 投影后 Item)：NDC z ∈ [-1,1]，越正越远，远的先画
    let mut drawn: Vec<(f64, Item)> = Vec::with_capacity(prims.len());
    for prim in prims {
        let pts = match prim {
            Prim3D::Polyline { pts, .. } | Prim3D::Polygon { pts, .. } => pts,
        };
        let min_pts = match prim {
            Prim3D::Polyline { .. } => 2,
            Prim3D::Polygon { .. } => 3,
        };
        if pts.len() < min_pts {
            continue;
        }
        // 齐次变换 + 手动除 w（判相机身后 + NDC）
        let mut ndc = Vec::with_capacity(pts.len());
        let mut behind = false;
        for p in pts {
            let c = vp.mul_vec4(DVec4::new(p.x, p.y, p.z, 1.0));
            if c.w <= 0.0 {
                behind = true;
                break;
            }
            ndc.push(DVec3::new(c.x / c.w, c.y / c.w, c.z / c.w));
        }
        if behind {
            continue;
        }
        // NDC → 像素：x 右不变，y 翻转（NDC 向上 → 屏幕向下）
        let to_px = |n: DVec3| Point::new((n.x + 1.0) * 0.5 * width, (1.0 - n.y) * 0.5 * height);
        let depth = ndc.iter().map(|n| n.z).sum::<f64>() / ndc.len() as f64;
        let mut path = BezPath::new();
        for (i, n) in ndc.iter().enumerate() {
            let px = to_px(*n);
            path.push(if i == 0 {
                PathEl::MoveTo(px)
            } else {
                PathEl::LineTo(px)
            });
        }
        let item = match prim {
            Prim3D::Polyline { style, .. } => Item::from_path(path)
                .with_stroke(style.stroke)
                .with_stroke_width(style.stroke_width),
            Prim3D::Polygon { style, .. } => {
                path.close_path();
                let mut it = Item::from_path(path).with_stroke_width(style.stroke_width);
                if let Some(fill) = style.fill {
                    it = it.with_fill(fill);
                }
                if style.stroke_width > 0.0 {
                    it = it.with_stroke(style.stroke);
                }
                it
            }
        };
        drawn.push((depth, item));
    }
    // 画家算法：远（z 大）在前，近的后画盖住远的；NaN 视为最远（保守画底层）
    drawn.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Less));
    drawn.into_iter().map(|(_, it)| it).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 平视机位：phi=90°（平视）、theta=0（相机在 +x 侧）、radius=5。
    fn cam() -> Camera3D {
        Camera3D {
            phi: 90f64.to_radians(),
            theta: 0.0,
            radius: 5.0,
            ..Default::default()
        }
    }

    /// 首个路径锚点（像素坐标）。
    fn first_pt(item: &Item) -> Point {
        match &item.shape {
            crate::Shape::Path(p) => match p.iter().next() {
                Some(PathEl::MoveTo(pt)) => pt,
                other => panic!("首段应为 MoveTo，实际 {other:?}"),
            },
            other => panic!("应为路径 Item，实际 {other:?}"),
        }
    }

    #[test]
    fn 相机_眼位球坐标() {
        // phi=90°、theta=0、radius=5 → 相机在 +x 轴上
        assert!((cam().eye() - DVec3::new(5.0, 0.0, 0.0)).length() < 1e-9);
        // phi=0 → 正上方俯视
        let top = Camera3D { phi: 0.0, ..cam() };
        assert!((top.eye() - DVec3::new(0.0, 0.0, 5.0)).length() < 1e-9);
    }

    #[test]
    fn 投影_光轴点落屏幕中心() {
        let line = polyline3(
            [DVec3::ZERO, DVec3::new(0.1, 0.0, 0.0)],
            PrimStyle::line(Color::new(255, 0, 0, 255), 1.0),
        );
        let items = project(&[line], &cam(), 800.0, 600.0);
        assert_eq!(items.len(), 1);
        let pt = first_pt(&items[0]);
        assert!((pt.x - 400.0).abs() < 1e-9 && (pt.y - 300.0).abs() < 1e-9);
    }

    #[test]
    fn 投影_世界z_up映射屏幕上方() {
        // target 上方点（+z）→ 屏幕 y 小于中心（屏幕上方）；+x（朝相机）→ 屏幕中心偏移
        let up = polyline3(
            [DVec3::new(0.0, 0.0, 1.0), DVec3::new(0.0, 0.0, 1.2)],
            PrimStyle::line(Color::new(255, 0, 0, 255), 1.0),
        );
        let items = project(&[up], &cam(), 800.0, 600.0);
        let pt = first_pt(&items[0]);
        assert!(pt.y < 300.0, "+z 应投影到屏幕上半: y={pt:?}");
    }

    #[test]
    fn 投影_画家算法远先画() {
        // 相机在 +x 看原点：面 A 中心 (1,0,0) 距相机 4（近），面 B (-1,0,0) 距 6（远）
        let sq = |cx: f64| {
            polygon3(
                [
                    DVec3::new(cx, -0.5, -0.5),
                    DVec3::new(cx, 0.5, -0.5),
                    DVec3::new(cx, 0.5, 0.5),
                    DVec3::new(cx, -0.5, 0.5),
                ],
                PrimStyle::face(Color::new(0, 0, 255, 255), Color::new(0, 0, 0, 255), 1.0),
            )
        };
        let items = project(&[sq(1.0), sq(-1.0)], &cam(), 800.0, 600.0);
        assert_eq!(items.len(), 2);
        // 近面（A，z 小）应排在输出末尾（后画）
        let a = first_pt(&items[1]);
        let b = first_pt(&items[0]);
        // B 远：投影更小、更靠近中心；A 近：更大更靠外——用 x 半径判即可
        assert!(
            (a.x - 400.0).abs() > (b.x - 400.0).abs(),
            "近面应更靠外: a={a:?} b={b:?}"
        );
    }

    #[test]
    fn 投影_相机身后图元丢弃() {
        // 相机在 (5,0,0) 朝原点看：点 (10,0,0) 在身后
        let back = polyline3(
            [DVec3::new(10.0, 0.0, 0.0), DVec3::new(12.0, 0.0, 0.0)],
            PrimStyle::line(Color::new(255, 0, 0, 255), 1.0),
        );
        let front = polyline3(
            [DVec3::ZERO, DVec3::new(0.5, 0.0, 0.0)],
            PrimStyle::line(Color::new(255, 0, 0, 255), 1.0),
        );
        let items = project(&[back, front], &cam(), 800.0, 600.0);
        assert_eq!(items.len(), 1, "仅前方折线存活");
    }

    #[test]
    fn 相机_俯视退化矩阵有限() {
        let top = Camera3D { phi: 0.0, ..cam() };
        let m = top.view_matrix();
        assert!(
            m.to_cols_array().iter().all(|v| v.is_finite()),
            "俯视矩阵应有限"
        );
        let line = polyline3(
            [DVec3::ZERO, DVec3::new(1.0, 0.0, 0.0)],
            PrimStyle::line(Color::new(255, 0, 0, 255), 1.0),
        );
        let items = project(&[line], &top, 800.0, 600.0);
        assert_eq!(items.len(), 1, "俯视投影不应 panic 或丢图元");
    }

    const RED: Color = Color::new(255, 0, 0, 255);
    const BLUE: Color = Color::new(0, 0, 255, 255);

    #[test]
    fn 轴系_图元数量与端点() {
        let opts = Axes3DOpts {
            size: DVec3::new(3.0, 2.0, 1.0),
            grid: 2,
            axis: RED,
            axis_width: 2.5,
            grid_line: BLUE,
            grid_width: 1.0,
        };
        let prims = axes3d(&opts);
        // 3 轴 + (2*2+1)*2 条网格线 = 13
        assert_eq!(prims.len(), 13);
        // x 轴端点 (3,0,0)；z 轴端点 (0,0,1)
        let Prim3D::Polyline { pts, style } = &prims[0] else {
            panic!("轴应为折线")
        };
        assert_eq!(pts[1], DVec3::new(3.0, 0.0, 0.0));
        assert_eq!(style.stroke, RED);
        // 网格线从第 4 条起，样式为网格色
        let Prim3D::Polyline { pts, style } = &prims[3] else {
            panic!("网格应为折线")
        };
        assert_eq!(pts[0], DVec3::new(-2.0, -2.0, 0.0));
        assert_eq!(style.stroke, BLUE);
    }

    #[test]
    fn 轴系_无网格() {
        let opts = Axes3DOpts {
            size: DVec3::ONE,
            grid: 0,
            axis: RED,
            axis_width: 1.0,
            grid_line: BLUE,
            grid_width: 1.0,
        };
        assert_eq!(axes3d(&opts).len(), 3, "grid=0 仅三轴");
    }

    #[test]
    fn 曲面_面片数与棋盘配色() {
        let style = SurfaceStyle::checkerboard(RED, BLUE, 1.0);
        // 平面 f(u,v)=(u,v,0)，u/v∈[0,1]，2×3 分辨率 → 6 面片
        let prims = parametric_surface(
            |u, v| DVec3::new(u, v, 0.0),
            (0.0, 1.0),
            (0.0, 1.0),
            (2, 3),
            &style,
        );
        assert_eq!(prims.len(), 6);
        let fills: Vec<Color> = prims
            .iter()
            .map(|p| match p {
                Prim3D::Polygon { style, .. } => style.fill.unwrap(),
                _ => panic!("应为面片"),
            })
            .collect();
        // (0,0) 面片取 a 色，(1,0) 取 b 色（(i+j)%2 交替）
        assert_eq!(fills[0], RED);
        assert_eq!(fills[1], BLUE);
        // (1,1) 面片（第 4 个）回到 a 色
        assert_eq!(fills[3], RED);
    }

    #[test]
    fn 曲面_球面采样与不透明度() {
        let style = SurfaceStyle::checkerboard(RED, BLUE, 0.5);
        // 半径 1.5 的上半球面
        let prims = parametric_surface(
            |u, v| 1.5 * DVec3::new(u.cos() * v.cos(), u.sin() * v.cos(), v.sin()),
            (0.0, std::f64::consts::TAU),
            (0.0, std::f64::consts::FRAC_PI_2),
            (4, 2),
            &style,
        );
        assert_eq!(prims.len(), 8);
        // 所有顶点距原点 ≈1.5（球面参数化正确）
        let all_on_sphere = prims.iter().all(|p| match p {
            Prim3D::Polygon { pts, .. } => pts.iter().all(|v| (v.length() - 1.5).abs() < 1e-9),
            _ => false,
        });
        assert!(all_on_sphere, "球面采样点应全部落在半径 1.5 上");
        // opacity 0.5 → alpha 128
        let Prim3D::Polygon { style, .. } = &prims[0] else {
            panic!("应为面片")
        };
        assert_eq!(style.fill.unwrap().a, 128);
    }
}
