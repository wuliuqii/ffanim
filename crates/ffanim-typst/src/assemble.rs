//! 装配片共享合并：Formula 全部片段 → 单 BezPath（arrow / decimal 共用）。
//!
//! delimiter 的装配含 DoubleBracket 中线拆分特例，留在本地；本模块只收
//! 「全合并 + 空校验」公共管线（同色不透明填充重叠区无瑕疵）。

use ffanim_core::kurbo::BezPath;
use ffanim_core::{Error, Result};

use super::formula::Formula;

/// 全部装配片合并为单路径；装配片为空时报错（`label` 定制错误文案）。
pub(crate) fn assemble(f: &Formula, label: &str) -> Result<BezPath> {
    let mut out = BezPath::new();
    for p in &f.pieces {
        for el in p.path.elements() {
            out.push(*el);
        }
    }
    if out.elements().is_empty() {
        return Err(Error::Other(format!("{label}装配片为空")));
    }
    Ok(out)
}
