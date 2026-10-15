//! ffanim 网页画廊生成器：examples 即数据源，场景清单零硬编码。
//!
//! 流程：扫描 crates/ffanim/examples/*.rs（标题 / SCENES 清单 / doc 摘要 / 源码全文）
//! → 逐场景调 `cargo run --release -p ffanim --example <ex> -- <scene>` 出 out/<scene>.mp4
//! → 生成 docs/gallery/index.html（卡片网格 + 视频 + 源码折叠，对齐 manim 官网画廊形态）。
//!
//! 用法（仓库根）：
//! `cargo run --release -p ffanim-gally`（全量渲染 + 生成）
//! `SKIP_RENDER=1 cargo run --release -p ffanim-gally`（只重建 HTML，复用已有 mp4）

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 单个场景：argv 寻址名 + doc 首行摘要。
struct Scene {
    name: String,
    doc: String,
    rendered: bool,
}

/// 一个 example 文件：标题 + 场景清单 + 源码全文。
struct Example {
    name: String,
    title: String,
    scenes: Vec<Scene>,
    source: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("CARGO_MANIFEST_DIR 应为 crates/ffanim-gally")
        .to_path_buf()
}

/// 解析单个 examples/*.rs：//! 标题、SCENES 数组、每场景 doc 首行。
fn parse_example(path: &Path) -> Option<Example> {
    let name = path.file_stem()?.to_str()?.to_string();
    let text = fs::read_to_string(path).ok()?;
    let mut title = String::new();
    let mut scenes: Vec<Scene> = Vec::new();
    let mut docs: Vec<(String, String)> = Vec::new(); // (fn 名, doc 首行)
    let mut pending_doc = String::new();
    let mut in_scenes = false;

    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("//!") {
            if title.is_empty() {
                title = t.trim_start_matches("//!").trim().to_string();
            }
        } else if t.starts_with("///") {
            if pending_doc.is_empty() {
                pending_doc = t.trim_start_matches("///").trim().to_string();
            }
        } else if t.starts_with("const SCENES") {
            // 数组体 = 首个 '=' 之后（类型标注 &[&str] 含 ']'，不能整体 contains 判闭合）
            let body = t
                .find('=')
                .map(|i| t[i + 1..].trim_start_matches([' ', '&', '[']).trim_start())
                .unwrap_or("");
            for lit in body.split('"').skip(1).step_by(2) {
                if !lit.is_empty() {
                    scenes.push(Scene {
                        name: lit.to_string(),
                        doc: String::new(),
                        rendered: false,
                    });
                }
            }
            in_scenes = !body.contains(']');
        } else if in_scenes {
            if t.starts_with(']') {
                in_scenes = false;
            } else {
                // 提取引号内场景名："name" 的奇数段
                for lit in t.split('"').skip(1).step_by(2) {
                    if !lit.is_empty() {
                        scenes.push(Scene {
                            name: lit.to_string(),
                            doc: String::new(),
                            rendered: false,
                        });
                    }
                }
            }
        } else if t.starts_with("fn ") || t.starts_with("pub fn ") {
            let fn_name = t
                .trim_start_matches("pub ")
                .trim_start_matches("fn ")
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !pending_doc.is_empty() {
                docs.push((fn_name, std::mem::take(&mut pending_doc)));
            }
        } else if t.is_empty() {
            // 空行断开 doc 块，防跨函数粘连
            pending_doc.clear();
        }
    }
    // doc 摘要回填到场景（按 fn 名 = 场景名）
    for sc in &mut scenes {
        sc.doc = docs
            .iter()
            .find(|(f, _)| *f == sc.name)
            .map(|(_, d)| d.clone())
            .unwrap_or_default();
    }
    Some(Example {
        name,
        title,
        scenes,
        source: text,
    })
}

/// 渲染单场景：子进程调 examples（复用其出片管线），产物 out/<scene>.mp4。
fn render_scene(example: &str, scene: &str) -> bool {
    Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "ffanim",
            "--example",
            example,
            "--",
            scene,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn gen_html(examples: &[Example]) -> String {
    let mut page = String::new();
    page.push_str("<!DOCTYPE html>\n<html lang=\"zh\">\n<head>\n<meta charset=\"utf-8\">\n");
    page.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    page.push_str("<title>ffanim Example Gallery</title>\n<style>\n");
    page.push_str(
        "body{font-family:system-ui,sans-serif;max-width:1080px;margin:0 auto;padding:16px;background:#fafafa;color:#222}\n\
         h1{border-bottom:2px solid #4a7dbd;padding-bottom:8px}\n\
         section{margin:32px 0}\n\
         h2{color:#4a7dbd}\n\
         .grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(480px,1fr));gap:16px}\n\
         .card{background:#fff;border:1px solid #ddd;border-radius:8px;padding:12px;box-shadow:0 1px 3px rgba(0,0,0,.08)}\n\
         .card h3{margin:0 0 4px;font-family:ui-monospace,monospace;font-size:15px;color:#333}\n\
         .card p{margin:4px 0 8px;font-size:13px;color:#666}\n\
         video{width:100%;border-radius:4px;background:#000}\n\
         .miss{color:#c0392b;font-size:13px}\n\
         details{margin-top:8px;font-size:13px}\n\
         pre{background:#f0f0f0;padding:10px;border-radius:4px;overflow:auto;max-height:400px;font-size:12px}\n",
    );
    page.push_str("</style>\n</head>\n<body>\n");
    page.push_str("<h1>ffanim Example Gallery</h1>\n");
    let total: usize = examples.iter().map(|e| e.scenes.len()).sum();
    page.push_str(&format!(
        "<p>{total} 个场景示例。复现任意场景：<code>cargo run -p ffanim --example &lt;文件名&gt; -- &lt;场景名&gt;</code>（对齐 manim example_scenes 形态）</p>\n"
    ));
    for ex in examples {
        page.push_str(&format!(
            "<section>\n<h2>{}</h2>\n<div class=\"grid\">\n",
            html_escape(&ex.title)
        ));
        for sc in &ex.scenes {
            page.push_str(&format!(
                "<div class=\"card\">\n<h3>{}</h3>\n<p>{}</p>\n",
                html_escape(&sc.name),
                html_escape(&sc.doc)
            ));
            if sc.rendered {
                page.push_str(&format!(
                    "<video controls loop muted preload=\"metadata\"><source src=\"../../out/{}.mp4\" type=\"video/mp4\"></video>\n",
                    sc.name
                ));
            } else {
                page.push_str(&format!(
                    "<p class=\"miss\">渲染产物缺失（out/{}.mp4）</p>\n",
                    sc.name
                ));
            }
            page.push_str("</div>\n");
        }
        page.push_str("</div>\n");
        page.push_str(&format!(
            "<details><summary>源码：{}.rs</summary>\n<pre><code>{}</code></pre>\n</details>\n</section>\n",
            ex.name,
            html_escape(&ex.source)
        ));
    }
    page.push_str("</body>\n</html>\n");
    page
}

fn main() -> std::io::Result<()> {
    let root = repo_root();
    let ex_dir = root.join("crates/ffanim/examples");
    let skip_render = std::env::var("SKIP_RENDER").is_ok();

    let mut paths: Vec<PathBuf> = fs::read_dir(&ex_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    paths.sort();
    let mut examples: Vec<Example> = paths.iter().filter_map(|p| parse_example(p)).collect();

    // 渲染：SKIP_RENDER=1 复用已有 mp4；产物缺失才调 examples 出片
    for ex in &mut examples {
        for sc in &mut ex.scenes {
            let mp4 = root.join(format!("out/{}.mp4", sc.name));
            if skip_render && mp4.exists() {
                sc.rendered = true;
                println!("跳过 {}（已有产物）", sc.name);
                continue;
            }
            print!("渲染 {}/{} ... ", ex.name, sc.name);
            use std::io::Write as _;
            std::io::stdout().flush().ok();
            sc.rendered = render_scene(&ex.name, &sc.name);
            println!("{}", if sc.rendered { "ok" } else { "失败" });
        }
    }

    let out_dir = root.join("docs/gallery");
    fs::create_dir_all(&out_dir)?;
    let total_rendered = examples
        .iter()
        .map(|e| e.scenes.iter().filter(|s| s.rendered).count())
        .sum::<usize>();
    let total: usize = examples.iter().map(|e| e.scenes.len()).sum();
    fs::write(out_dir.join("index.html"), gen_html(&examples))?;
    println!("已生成 docs/gallery/index.html（{total_rendered}/{total} 场景有视频）");
    Ok(())
}
