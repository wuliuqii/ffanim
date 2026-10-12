//! `ffanim new` 脚手架：从内嵌模板生成最小可跑项目（空目录 → 出片）。

use std::path::{Path, PathBuf};

use ffanim_core::{Error, Result};

/// 场景模板注册表（名 → 模板 main.rs）。
const MAIN_TEMPLATES: &[(&str, &str)] = &[
    ("minimal", include_str!("../templates/main_minimal.rs")),
    ("coords", include_str!("../templates/main_coords.rs")),
];

const CARGO_TMPL: &str = include_str!("../templates/Cargo.toml.tmpl");
const GITIGNORE: &str = include_str!("../templates/gitignore.tmpl");
const THEME_JSON: &str = include_str!("../../../themes/default.json");

/// 模板清单（--help 提示用）。
pub fn template_names() -> Vec<&'static str> {
    MAIN_TEMPLATES.iter().map(|(n, _)| *n).collect()
}

/// 生成项目到 `dir`：Cargo.toml / src/main.rs / themes/default.json / .gitignore。
///
/// `dev = true` 时 ffanim 依赖直接指向本仓库源码（本机验证全链路用）。
/// 返回生成的文件列表。
pub fn generate(dir: &Path, template: &str, dev: bool) -> Result<Vec<PathBuf>> {
    let main_src = MAIN_TEMPLATES
        .iter()
        .find(|(n, _)| *n == template)
        .map(|(_, src)| *src)
        .ok_or_else(|| {
            Error::Other(format!(
                "未知模板 {template:?}，可用: {}",
                template_names().join(", ")
            ))
        })?;

    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::Other(format!("非法目录名: {}", dir.display())))?;

    // Cargo.toml：替换项目名；dev 模式换成仓库 path 依赖
    let mut cargo = CARGO_TMPL.replace("{{name}}", name);
    if dev {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("CLI 位于 <仓库>/crates/ffanim-cli");
        cargo = cargo.replace(
            r#"ffanim = "0.1""#,
            &format!(r#"ffanim = {{ path = {:?} }}"#, repo.join("crates/ffanim")),
        );
    }

    let files: Vec<(PathBuf, &str)> = vec![
        (dir.join("Cargo.toml"), cargo.as_str()),
        (dir.join("src/main.rs"), main_src),
        (dir.join("themes/default.json"), THEME_JSON),
        (dir.join(".gitignore"), GITIGNORE),
    ];
    for (path, content) in &files {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::ThemeIo(e, parent.to_path_buf()))?;
        }
        std::fs::write(path, content).map_err(|e| Error::ThemeIo(e, path.clone()))?;
    }
    log::info!(
        "脚手架完成: {}（模板 {template}，dev={dev}），{} 个文件",
        dir.display(),
        files.len()
    );
    Ok(files.into_iter().map(|(p, _)| p).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// minimal 模板生成 4 个文件，内容含关键 API 调用。
    #[test]
    fn 生成最小项目() {
        let dir = std::env::temp_dir().join(format!("ffanim_new_test_{}", std::process::id()));
        let files = generate(&dir, "minimal", false).expect("生成失败");
        assert_eq!(files.len(), 4);
        let main = std::fs::read_to_string(dir.join("src/main.rs")).unwrap();
        assert!(main.contains("render_to_mp4"), "模板应调用一键出片 API");
        let cargo = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(cargo.contains(r#"name = ""#) && cargo.contains(r#"ffanim = "0.1""#));
        let theme = std::fs::read_to_string(dir.join("themes/default.json")).unwrap();
        assert!(theme.contains("background"), "主题 JSON 应已嵌入");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 未知模板报错且信息含可用清单。
    #[test]
    fn 未知模板报错() {
        let err = generate(Path::new("/tmp/ffanim_x"), "nope", false).unwrap_err();
        assert!(
            err.to_string().contains("minimal"),
            "错误信息应列出可用模板"
        );
    }
}
