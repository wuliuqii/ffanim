# 配方 FAQ

进阶效果的「怎么做」速查。ffanim 的设计原则是**最小原语**——下面这些
效果没有专用类，全部由基础词汇组合而成（每个配方都来自官方示例的
真实源码，可整段抄走改参数）。

## 让物体身后留下轨迹

**思路**：一个 tracker 管进度，两个 updater 分别重建「动点」和
「已走过的轨迹」。轨迹 = 起点**截断到当前进度**的参数化折线，
每帧整体重绘——「走过就留下」自然成立。

```rust
// 节选自 trace_demo：方形轨迹按周长参数化 t∈[0,1)
let t = tl.add_tracker(0.0);
// 动点：tracker 当前值即位置
let _dot = tl.add_updater(&[t], move |v| {
    Item::circle(Circle::new(square_pt(v[0]), 10.0)).with_fill(accent)
});
// 轨迹：从起点重建到当前进度（N 等分，走多少画多少）
let _trace = tl.add_updater(&[t], move |v| {
    let n = (v[0] * N as f64).ceil() as usize;
    let mut path = BezPath::new();
    path.move_to(square_pt(0.0));
    for i in 1..=n { path.line_to(square_pt(i as f64 / N as f64)); }
    Item::from_path(path).with_stroke(fg).with_stroke_width(2.5)
});
tl.play(Anim::set_value(t, 1.0).during(6.0).rate(RateFunc::Linear));
```

完整源码：[trace_demo](basic.md#trace_demo)

## 画中画：局部放大镜

**思路**：零新增 API 的**出片层拼装**——主帧照常渲染；克隆一份，
所有对象左乘「源中心 → 窗口中心」的放大仿射，渲染成窗口尺寸小帧
（画布边界天然裁剪，不需要 clip）；最后 blit 回主帧 + 描边框。

```rust
// 节选自 pip.rs（出片层，每帧调用）
let draw = render_frame(ct, frame);
let mut buf = CpuBackend::new().render(&draw);
// 放大仿射：源中心 src_c 映射到窗口中心
let m = Affine::translate((f64::from(PIP_W) / 2.0 - ZOOM * src_c.x,
                           f64::from(PIP_H) / 2.0 - ZOOM * src_c.y))
    * Affine::scale(ZOOM);
let mut pip = draw;
pip.width = PIP_W; pip.height = PIP_H;   // 必须改成窗口尺寸，否则行距撕裂
for it in &mut pip.items { it.transform = m * it.transform; }
let pb = CpuBackend::new().render(&pip);
// blit 回主帧 + stroke_rect 画边框（略）
```

完整源码：[pip](pip.md)

## 布尔运算：图形挖洞 / 拼接

**思路**：`Item::boolean(a, b, BoolOp::...)` 一个原语搞定四则
（`Intersection` / `Union` / `Xor` / `Difference`），结果是普通闭合
路径——所有现有动画（create / transform / 填充）直接可用。

```rust
// 节选自 boolean_demo：大圆挖掉小圆（月牙），照常参与动画
let moon = tl.add(
    big.boolean(&small, ffanim::BoolOp::Difference)
        .with_fill(accent)
        .with_stroke(fg),
);
tl.play(Anim::create(moon).during(1.0));
```

完整源码：[boolean_demo](basic.md#boolean_demo)

## 网格逐点形变（apply_function）

**思路**：把网格拆成折线集合，每条折线按自定义函数重算终点
（`Axes::c2p` 负责坐标换算），updater 每帧重建——manim
`apply_function` 的语义就是「逐点映射 + 每帧重画」。

```rust
// 节选自 warp_grid_demo：横竖线端点经 fn 逐点重算
let _grid = tl.add_updater(&[t], move |v| {
    let mut lines: Vec<Item> = Vec::new();
    for row in rows {                       // 预生成的折线端点集
        let pts: Vec<Point> = row.iter().map(|p| warp(*p, v[0])).collect();
        lines.push(polyline_item(&pts));
    }
    Item::group(lines)
});
```

完整源码：[warp_grid_demo](basic.md#warp_grid_demo)

## 相机跟着物体走

**思路**：相机与物体**同窗** `move_along_path`。注意相机变换是
world→screen 仿射，位移要左乘取反并考虑 zoom 放大——相机路径点取
`r_i = (1+z)·q0 − z·q_i`（q 为物体路径），注视点就恒锁物体。

```rust
// 节选自 follow_cam_demo：z=1.5 时物体与相机的补偿路径
tl.together([
    Anim::move_along_path(dot, &dot_path).during(4.0).rate(RateFunc::Linear),
    Anim::move_along_path(camera, &cam_path).during(4.0).rate(RateFunc::Linear),
]);
```

完整源码：[follow_cam_demo](basic.md#follow_cam_demo)（含补偿公式推导）

## 数字跟着数据滚动 + 图形联动重绘

**思路**：双 tracker（宽、高）驱动两个 updater——一个每帧重建
矩形与轴上拖点，一个把 `v[0]·v[1]` 乘积喂给 `decimal_signal`
（DecimalNumber 数字滚动）。

```rust
// 节选自 polygon_axes_demo：面积 = 宽×高，数字与矩形同窗联动
let sig = TypstEngine::decimal_signal(28.0, fg);
let area = tl.add_updater(&[rw, rh], move |v| sig(&[v[0] * v[1]]));
```

完整源码：[polygon_axes_demo](basic.md#polygon_axes_demo)

## 还没有想要的配方？

- 翻[场景画廊](basic.md)——21 个官方场景覆盖 manim gallery 全部形态
- 按[效果画廊](effect-gallery.md)的「你想要 → 用什么」表反查词汇
- 查[词汇速查](vocabulary.md)
