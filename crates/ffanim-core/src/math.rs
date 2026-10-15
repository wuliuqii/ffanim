//! 数值计算工具集（docs/07 §2.1 ODE 积分 + §2.4 求根 + §2.2 3D 线代件）。
//!
//! 定位：构建期一次性积分出轨迹、渲染期纯查表——与两阶段时间轴模型一致，
//! 渲染期零数值计算。典型用法：`rk4` 预积分单摆/抛体 → [`Trajectory::at`]
//! 按帧时间取状态 → updater 逐帧重建 Item。
//!
//! 3D 线代件直接采用 glam（f64 全套：零传递依赖、仅启用 f64 精度类型、
//! 锁小版本；docs/07 §2.2 修正案），在此 re-export 为 ffanim 的稳定数学面：
//! `DVec3`/`DMat4`/`DQuat`。约定（右手系、OpenGL 深度 [-1,1]，manim 对齐）
//! 由下方测试锁定——约定漂移即 CI 红。

pub use glam::{DMat4, DQuat, DVec3};

// 相机/投影构造（glam 0.30+ 移入 camera 模块且按手系 × NDC 约定分发）：
// ffanim 在此**锁死约定**——右手系（manim 同款）+ OpenGL 深度 [-1,1]（示教对齐），
// 上层 1.2 投影管线只消费这些稳定别名，不再面对 rh/lh、opengl/directx/vulkan 的选择题。
/// 正交投影（同上约定，glOrtho 等价；near/far 为视空间距离）。
pub use glam::dcamera::rh::proj::opengl::orthographic;
/// 透视投影（右手系视空间 → OpenGL NDC 深度 [-1,1]）。
pub use glam::dcamera::rh::proj::opengl::perspective;
/// 右手系 look-at 视图矩阵（视空间 Y-up，相机朝 −z 看）。
pub use glam::dcamera::rh::view::look_at_mat4 as look_at;

/// 四阶 Runge-Kutta 预积分：`dy/dt = f(t, y)`，`y ∈ R^n`，均匀步长
/// `[t0, t1]` 共 `steps` 步（含两端点，共 `steps + 1` 个采样）。
///
/// 状态量用 `&[f64]`/`Vec<f64>`（单摆 `[θ, ω]`、抛体 `[x, y, vx, vy]`、
/// 洛伦兹 `[x, y, z]`）；教学场景规模下分配开销可忽略。
pub fn rk4(
    f: impl Fn(f64, &[f64]) -> Vec<f64>,
    y0: &[f64],
    t0: f64,
    t1: f64,
    steps: usize,
) -> Trajectory {
    assert!(steps > 0, "rk4 至少 1 步");
    assert!(!y0.is_empty(), "状态量维度不能为 0");
    let h = (t1 - t0) / steps as f64;
    assert!(h > 0.0, "t1 必须大于 t0");
    let mut t = Vec::with_capacity(steps + 1);
    let mut states = Vec::with_capacity(steps + 1);
    let mut y = y0.to_vec();
    t.push(t0);
    states.push(y.clone());
    let mut tmp = vec![0.0f64; y.len()];
    for i in 0..steps {
        let ti = t0 + i as f64 * h;
        let k1 = f(ti, &y);
        for (tmp_i, (y_i, k1_i)) in tmp.iter_mut().zip(y.iter().zip(&k1)) {
            *tmp_i = y_i + 0.5 * h * k1_i;
        }
        let k2 = f(ti + 0.5 * h, &tmp);
        for (tmp_i, (y_i, k2_i)) in tmp.iter_mut().zip(y.iter().zip(&k2)) {
            *tmp_i = y_i + 0.5 * h * k2_i;
        }
        let k3 = f(ti + 0.5 * h, &tmp);
        for (tmp_i, (y_i, k3_i)) in tmp.iter_mut().zip(y.iter().zip(&k3)) {
            *tmp_i = y_i + h * k3_i;
        }
        let k4 = f(ti + h, &tmp);
        for ((y_i, k1_i), ((k2_i, k3_i), k4_i)) in
            y.iter_mut().zip(&k1).zip(k2.iter().zip(&k3).zip(&k4))
        {
            *y_i += h / 6.0 * (k1_i + 2.0 * k2_i + 2.0 * k3_i + k4_i);
        }
        t.push(ti + h);
        states.push(y.clone());
    }
    Trajectory { t, states }
}

/// 预积分轨迹：均匀时间网格 + 状态采样，渲染期 `at(t)` 线性插值查表。
#[derive(Debug, Clone)]
pub struct Trajectory {
    /// 时间网格（均匀，升序）。
    pub t: Vec<f64>,
    /// 各时间点的状态采样（与 `t` 等长）。
    pub states: Vec<Vec<f64>>,
}

impl Trajectory {
    /// 帧时间 → 状态（线性插值；越界钳位到端点——动画窗口常比积分窗口长）。
    pub fn at(&self, time: f64) -> Vec<f64> {
        let n = self.t.len();
        let first = self.t[0];
        let last = self.t[n - 1];
        if time <= first {
            return self.states[0].clone();
        }
        if time >= last {
            return self.states[n - 1].clone();
        }
        // 均匀网格：直接算索引，无需二分
        let h = (last - first) / (n - 1) as f64;
        let i = (((time - first) / h) as usize).min(n - 2);
        let frac = (time - self.t[i]) / h;
        self.states[i]
            .iter()
            .zip(&self.states[i + 1])
            .map(|(a, b)| a + (b - a) * frac)
            .collect()
    }

    /// 某状态分量的 (t, y) 时间序列——喂 `Axes::plot` / 相图曲线。
    pub fn component(&self, k: usize) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.t
            .iter()
            .copied()
            .zip(self.states.iter().map(move |s| s[k]))
    }

    /// 时间网格长度（采样点数）。
    pub fn len(&self) -> usize {
        self.t.len()
    }

    /// 是否为空（rk4 恒非空，防御式保留）。
    pub fn is_empty(&self) -> bool {
        self.t.is_empty()
    }
}

/// 二分法求根（docs/07 §2.4：曲线交点标注、切线定位）。
/// 要求 `f(a)` 与 `f(b)` 异号，否则返回 `None`；迭代至区间宽 < `tol`。
pub fn bisect(f: impl Fn(f64) -> f64, a: f64, b: f64, tol: f64) -> Option<f64> {
    let (mut lo, mut hi) = (a, b);
    let (flo, fhi) = (f(lo), f(hi));
    if flo == 0.0 {
        return Some(lo);
    }
    if fhi == 0.0 {
        return Some(hi);
    }
    if flo * fhi > 0.0 {
        return None;
    }
    while hi - lo > tol {
        let mid = 0.5 * (lo + hi);
        let fm = f(mid);
        if fm == 0.0 {
            return Some(mid);
        }
        if flo * fm < 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 抛体运动解析解对照：x=v·t, y=v·t−g/2·t²。
    #[test]
    fn rk4_抛体与解析解吻合() {
        let g = 9.8;
        let (v0, angle) = (10.0, 45f64.to_radians());
        let traj = rk4(
            |_t, y| vec![y[2], y[3], 0.0, -g],
            &[0.0, 0.0, v0 * angle.cos(), v0 * angle.sin()],
            0.0,
            1.0,
            100,
        );
        for &t in &[0.25, 0.5, 0.75] {
            let s = traj.at(t);
            assert!(
                (s[0] - v0 * angle.cos() * t).abs() < 1e-6,
                "x({t}) 偏差过大"
            );
            assert!((s[1] - (v0 * angle.sin() * t - 0.5 * g * t * t)).abs() < 1e-6);
        }
        // 越界钳位
        assert_eq!(traj.at(-1.0), traj.at(0.0));
        assert_eq!(traj.at(2.0), traj.at(1.0));
    }

    /// 单摆能量守恒冒烟：无阻尼摆总能量 E = ½l²ω² + gl(1−cosθ) 恒定。
    #[test]
    fn rk4_单摆无阻尼能量守恒() {
        let (g, l) = (9.8, 1.0);
        let traj = rk4(
            |_t, y| vec![y[1], -(g / l) * y[0].sin()],
            &[0.5, 0.0],
            0.0,
            10.0,
            1000,
        );
        let energy = |s: &[f64]| 0.5 * l * l * s[1] * s[1] + g * l * (1.0 - s[0].cos());
        let e0 = energy(&traj.states[0]);
        let drift = traj
            .states
            .iter()
            .map(|s| (energy(s) - e0).abs())
            .fold(0.0f64, f64::max);
        assert!(drift < 1e-4, "无阻尼单摆 10s 能量漂移过大: {drift}");
        // 周期性：t=10s（≈1.59 个周期）状态不会飘走，θ 仍被约束在摆幅内
        let theta_max = traj
            .states
            .iter()
            .map(|s| s[0].abs())
            .fold(0.0f64, f64::max);
        assert!(theta_max < 0.5 + 1e-3, "θ 超出释放摆幅: {theta_max}");
    }

    #[test]
    fn bisect_求根与异号检查() {
        // √2 在 [1, 2]
        let root = bisect(|x| x * x - 2.0, 1.0, 2.0, 1e-10).unwrap();
        assert!((root - 2.0f64.sqrt()).abs() < 1e-9);
        // 同号区间 → None
        assert!(bisect(|x| x * x + 1.0, 0.0, 1.0, 1e-6).is_none());
    }

    // ---- 3D 线代件（docs/07 §2.2）：glam 约定锁定 ----
    // 目的：锁住 ffanim 依赖的三方约定（右手系 + OpenGL 深度 [-1,1]），
    // 升级 glam 小版本时约定漂移即此处红，1.2 投影管线才能安全叠在其上。

    /// 叉积右手性：x × y = z（3D 场景手系基线，精确 0/1 值无舍入）。
    #[test]
    fn 线代_叉积右手性() {
        assert_eq!(DVec3::X.cross(DVec3::Y), DVec3::Z);
    }

    /// z 轴旋转 90°：x 轴单位向量转到 y 轴（右手系，+z 朝观察者逆时针）。
    #[test]
    fn 线代_旋转矩阵右手约定() {
        let m = DMat4::from_rotation_z(90f64.to_radians());
        let r = m.transform_vector3(DVec3::X);
        assert!((r - DVec3::Y).length() < 1e-9);
    }

    /// look_at 右手视空间：相机 (0,0,5) 看原点，原点在视空间 z = −5
    /// （右手系相机朝 −z 看——1.2 相机的 view 矩阵语义）。
    #[test]
    fn 线代_look_at右手视空间() {
        let view = look_at(DVec3::new(0.0, 0.0, 5.0), DVec3::ZERO, DVec3::Y);
        let v = view.transform_point3(DVec3::ZERO);
        assert!((v - DVec3::new(0.0, 0.0, -5.0)).length() < 1e-9);
    }

    /// 透视投影（OpenGL 深度 [-1,1]，manim/示教对齐）：
    /// 近平面 → NDC z=−1、远平面 → +1；光轴上点投影到 NDC 中心。
    ///
    /// 坑：透视链路必须用 `project_point3`（含 w 除法）——`transform_point3`
    /// 是仿射专用（断言 w 行 = (0,0,0,1) 且丢弃 w），对投影矩阵返回的是
    /// clip 空间 z 而非 NDC。1.2 投影管线一律走 `project_point3`。
    #[test]
    fn 线代_透视投影深度约定() {
        let proj = perspective(90f64.to_radians(), 1.0, 0.1, 10.0);
        let near = proj.project_point3(DVec3::new(0.0, 0.0, -0.1)).z;
        let far = proj.project_point3(DVec3::new(0.0, 0.0, -10.0)).z;
        assert!((near + 1.0).abs() < 1e-9, "近平面应映射 NDC z=−1: {near}");
        assert!((far - 1.0).abs() < 1e-9, "远平面应映射 NDC z=+1: {far}");
        let c = proj.project_point3(DVec3::new(0.0, 0.0, -5.0));
        assert!(
            c.x.abs() < 1e-9 && c.y.abs() < 1e-9,
            "光轴点应投影到 NDC 中心"
        );
    }

    /// 正交投影（同 OpenGL 深度约定）：近/远边界映射 ±1。
    #[test]
    fn 线代_正交投影边界() {
        let o = orthographic(-1.0, 1.0, -1.0, 1.0, 1.0, 10.0);
        let znear = o.project_point3(DVec3::new(0.0, 0.0, -1.0)).z;
        let zfar = o.project_point3(DVec3::new(0.0, 0.0, -10.0)).z;
        assert!((znear + 1.0).abs() < 1e-9, "近平面 z=−1: {znear}");
        assert!((zfar - 1.0).abs() < 1e-9, "远平面 z=+1: {zfar}");
    }

    /// 四元数与旋转矩阵作用一致（相机 orbit 组合时两者可互换）。
    #[test]
    fn 线代_四元数与矩阵一致() {
        let q = DQuat::from_rotation_y(90f64.to_radians());
        let m = DMat4::from_rotation_y(90f64.to_radians());
        let vq = q * DVec3::X;
        let vm = m.transform_vector3(DVec3::X);
        assert!((vq - vm).length() < 1e-9);
    }
}
