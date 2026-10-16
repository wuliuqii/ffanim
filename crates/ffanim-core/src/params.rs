//! 场景参数加载（docs/07 §5.2）：typed struct 即 schema，JSON overlay 只覆盖要改的字段。
//!
//! schema/缺省值/类型校验三职责全部落在场景侧参数结构体上（serde 容器级
//! `#[serde(default)]` + `#[serde(deny_unknown_fields)]`），本模块只提供两个
//! 加载入口并统一错误出口——校验在加载点报错（Theme 同款解析前置约定），
//! 不泄漏 serde 错误到调用点。字段级整体替换，非深合并。

use std::path::Path;

use serde::de::DeserializeOwned;

use crate::{Error, Result};

/// 从 JSON 字符串加载场景参数（inline / 测试用）。
///
/// `P` 须实现 `Default`（缺省值）与 `DeserializeOwned`（schema）；
/// JSON 只写要覆盖的字段，缺失字段回退 `P::default()` 对应值。
pub fn params_from_str<P: DeserializeOwned + Default>(json: &str) -> Result<P> {
    serde_json::from_str(json).map_err(Error::ParamsParse)
}

/// 从 JSON 文件加载场景参数（overlay 语义：只写要覆盖的字段）。
pub fn load_params<P: DeserializeOwned + Default>(path: impl AsRef<Path>) -> Result<P> {
    let path = path.as_ref();
    let json = std::fs::read_to_string(path).map_err(|e| Error::ParamsIo(e, path.to_path_buf()))?;
    params_from_str(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    /// 测试用参数结构体（场景侧声明范式缩影）。
    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    struct DemoParams {
        /// 摆长（米）。
        length: f64,
        /// 初始摆角（度）。
        theta0_deg: f64,
        /// 摆动时长（秒）。
        duration: f64,
    }

    impl Default for DemoParams {
        fn default() -> Self {
            Self {
                length: 1.0,
                theta0_deg: 40.0,
                duration: 8.0,
            }
        }
    }

    #[test]
    fn 空对象回退全缺省() {
        let p: DemoParams = params_from_str("{}").expect("空对象全缺省");
        assert_eq!(p, DemoParams::default());
    }

    #[test]
    fn 部分覆盖其余回退缺省() {
        let p: DemoParams = params_from_str(r#"{"length": 2.5}"#).expect("单字段覆盖");
        assert_eq!(p.length, 2.5);
        assert_eq!(p.theta0_deg, 40.0, "未覆盖字段回退缺省");
        assert_eq!(p.duration, 8.0);
    }

    #[test]
    fn 未知字段加载点报错防typo() {
        // typo 静默失效是 overlay 语义的最大坑，deny_unknown_fields 必须拦
        let err = params_from_str::<DemoParams>(r#"{"lenght": 2.5}"#).expect_err("typo 字段报错");
        assert!(
            err.to_string().contains("unknown field"),
            "错误消息应含 unknown field: {err}"
        );
    }

    #[test]
    fn 类型错加载点报错() {
        let err = params_from_str::<DemoParams>(r#"{"length": "很长的摆"}"#)
            .expect_err("字符串喂 f64 报错");
        assert!(
            err.to_string().contains("invalid type"),
            "错误消息应含 invalid type: {err}"
        );
    }

    #[test]
    fn 文件缺失报io错() {
        let err =
            load_params::<DemoParams>("/nonexistent/params.json").expect_err("文件不存在报错");
        assert!(
            matches!(err, Error::ParamsIo(..)),
            "应为 ParamsIo 变体: {err:?}"
        );
    }
}
