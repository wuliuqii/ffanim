//! ffanim 书源生成器：examples 元数据 → mdBook 书源（docs/book/src/）。
//!
//! 流程：扫描 crates/ffanim/examples/*.rs（//! 标题 / SCENES 清单 / 每场景 doc 摘要）
//! → 渲染缺失的 out/<场景>.mp4（子进程复用 examples 出片管线）
//! → 拷贝产物到 docs/book/src/media/ → 生成 SUMMARY.md 与画廊章节 .md。
//!
//! **混合模式**：只有登记为画廊章节的 example 才生成页面（SUMMARY Part
//! 「场景画廊」之下）；素材文件（`Asset`，如入门示例 getting_started）只
//! 渲染出片 + 拷媒体，正文由手写教程页引用（`{{#include}}` / `<video>`），
//! 生成器永不触碰手写页。
//!
//! 画廊章节页格式：`## 场景名` + doc 摘要 + `<video>` + `<details>` 折叠的
//! `{{#include}}` 锚点代码段（examples 源文件中以 `// ANCHOR: 场景名` /
//! `// ANCHOR_END` 标注函数体）。
//! 无 SCENES 常量的创作示例（fourier/media/pip）视为单场景：整文件 include。
//!
//! 用法（仓库根）：
//! `cargo run --release -p ffanim-gally`（渲染缺失产物 + 生成书源）
//! `SKIP_RENDER=1 cargo run --release -p ffanim-gally`（只生成书源，复用已有 mp4）
//!
//! 构建：`just book`（= 本生成器 + `mdbook build docs/book`）。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 单个场景：argv 寻址名 + doc 首行摘要。
struct Scene {
    name: String,
    doc: String,
    rendered: bool,
}

/// 一个 example 文件：标题 + 场景清单 + 源文件相对路径。
struct Example {
    /// 文件名 stem（= cargo example 名）
    name: String,
    /// //! 头部首行（章节 H1）
    title: String,
    scenes: Vec<Scene>,
    /// 无 SCENES 常量的创作示例：单场景、渲染不带场景参数、整文件 include
    single: bool,
}

/// 章节类型：画廊章节生成页面，素材文件只出片供手写页引用。
enum ChapterKind {
    /// 生成画廊章节页 + SUMMARY 条目（order 定呈现序，label 为短标签）。
    Gallery(usize, &'static str),
    /// 素材文件：只渲染出片 + 拷媒体（场景名 = 文件名），不生成任何页面。
    Asset,
}

/// example 文件的章节归属（examples 是场景清单的唯一真相源）。
fn chapter_kind(stem: &str) -> ChapterKind {
    match stem {
        // 入门素材：正文由 first-animation.md 逐行讲解，不生成章节页
        "getting_started" => ChapterKind::Asset,
        // 分课教程素材：正文由六课手写页 lesson-*.md 引用（ANCHOR = 课名）
        "lessons" => ChapterKind::Asset,
        "basic" => ChapterKind::Gallery(0, "基础动画"),
        "typography" => ChapterKind::Gallery(1, "文字与公式"),
        "assemblies" => ChapterKind::Gallery(2, "装配件"),
        "tables" => ChapterKind::Gallery(3, "表格"),
        "fourier" => ChapterKind::Gallery(4, "创作示例：傅里叶级数"),
        "media" => ChapterKind::Gallery(5, "创作示例：SVG 与图片"),
        "pip" => ChapterKind::Gallery(6, "创作示例：画中画"),
        // 3D 对照账五例（docs/07 §1.2 manim gallery 3D 5/5）+ 原语验收创作例
        "lens" => ChapterKind::Gallery(7, "创作示例：放大镜透镜"),
        "pendulum" => ChapterKind::Gallery(8, "创作示例：单摆参数化"),
        "three_d_rotation" => ChapterKind::Gallery(9, "3D：相机环境自转"),
        "three_d_illusion" => ChapterKind::Gallery(10, "3D：相机错觉振荡"),
        "three_d_light" => ChapterKind::Gallery(11, "3D：棋盘光照"),
        "surface_plot" => ChapterKind::Gallery(12, "3D：曲面绘制"),
        "fixed_in_frame" => ChapterKind::Gallery(13, "3D：固定屏上层"),
        // 未登记的新 example 追加在后（字母序），场景清单仍自动提取
        _ => ChapterKind::Gallery(usize::MAX, ""),
    }
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
    let mut has_scenes = false;
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
            has_scenes = true;
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
    let single = !has_scenes;
    if single {
        // 创作示例：场景名 = 文件名（产物 out/<stem>.mp4）
        scenes.push(Scene {
            name: name.clone(),
            doc: String::new(),
            rendered: false,
        });
    }
    Some(Example {
        name,
        title,
        scenes,
        single,
    })
}

/// 渲染单场景：子进程调 examples（复用其出片管线），产物 out/<scene>.mp4。
fn render_scene(example: &str, scene: Option<&str>) -> bool {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--release", "-p", "ffanim", "--example", example]);
    if let Some(s) = scene {
        cmd.arg(s);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// 章节页：H1（//! 标题）+ 运行提示 + 逐场景（视频 + doc 摘要 + 锚点代码段）。
fn gen_chapter_md(ex: &Example) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", ex.title));
    if ex.single {
        md.push_str(&format!(
            "运行：`cargo run -p ffanim --example {}`\n\n",
            ex.name
        ));
    } else {
        md.push_str(&format!(
            "运行：`cargo run -p ffanim --example {} -- [场景名...]`（缺省全渲染，≙ `manim -a`）\n\n",
            ex.name
        ));
    }
    for sc in &ex.scenes {
        md.push_str(&format!("## {}\n\n", sc.name));
        if !sc.doc.is_empty() {
            md.push_str(&format!("{}\n\n", sc.doc));
        }
        if sc.rendered {
            md.push_str(&format!(
                "<video controls loop muted preload=\"metadata\" src=\"media/{}.mp4\"></video>\n\n",
                sc.name
            ));
        } else {
            md.push_str(&format!("> 渲染产物缺失（out/{}.mp4）\n\n", sc.name));
        }
        // include 相对路径：docs/book/src/ → 仓库根需三层上跳；全量源码折叠
        // （<details> 开标签后空行断开 HTML 块，内部 markdown/代码围栏正常解析）
        md.push_str("<details>\n<summary>完整源码</summary>\n\n");
        if ex.single {
            md.push_str(&format!(
                "```rust\n{{{{#include ../../../crates/ffanim/examples/{}.rs}}}}\n```\n\n",
                ex.name
            ));
        } else {
            md.push_str(&format!(
                "```rust\n{{{{#include ../../../crates/ffanim/examples/{}.rs:{}}}}}\n```\n\n",
                ex.name, sc.name
            ));
        }
        md.push_str("</details>\n\n");
    }
    md
}

/// SUMMARY.md：Part 骨架（手写页固定条目）+ 生成画廊章节 + 手写附录。
///
/// Part 标题（`# …` 行）为 mdBook 原生语法；手写页（index / effect-gallery /
/// first-animation / lesson-*.md 六课 / fourier-walkthrough / recipes /
/// vocabulary / manim-gallery）由人维护，生成器只填「场景画廊」一组。
fn gen_summary_md(chapters: &[(String, String)]) -> String {
    let mut s = String::from(
        "# Summary\n\n# 认识 ffanim\n\n- [介绍](index.md)\n- [效果画廊](effect-gallery.md)\n\n# 动手写动画\n\n- [第一个动画](first-animation.md)\n\n# 分课教程\n\n- [第 1 课 · 形状与样式](lesson-shapes.md)\n- [第 2 课 · 动画词汇](lesson-anims.md)\n- [第 3 课 · 文字与公式](lesson-text.md)\n- [第 4 课 · 坐标系与函数图像](lesson-axes.md)\n- [第 5 课 · 布局与强调](lesson-layout.md)\n- [第 6 课 · 镜头语言](lesson-camera.md)\n\n# 场景画廊\n\n",
    );
    for (file, label) in chapters {
        if label.is_empty() {
            s.push_str(&format!("- [{file}]({file}.md)\n"));
        } else {
            s.push_str(&format!("- [{label}]({file}.md)\n"));
        }
    }
    s.push_str(
        "\n# 实战与参考\n\n- [实战拆解：傅里叶级数](fourier-walkthrough.md)\n- [配方 FAQ](recipes.md)\n- [词汇速查](vocabulary.md)\n- [附录：与 manim gallery 对照](manim-gallery.md)\n",
    );
    s
}

fn main() -> std::io::Result<()> {
    let root = repo_root();
    let ex_dir = root.join("crates/ffanim/examples");
    let book_src = root.join("docs/book/src");
    let media_dir = book_src.join("media");
    let skip_render = std::env::var("SKIP_RENDER").is_ok();

    let mut paths: Vec<PathBuf> = fs::read_dir(&ex_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    // 呈现顺序：画廊章节按登记序优先，未登记文件按文件名排后（素材文件排最后）
    paths.sort_by_key(|p| {
        let stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let order = match chapter_kind(&stem) {
            ChapterKind::Gallery(order, _) => order,
            ChapterKind::Asset => usize::MAX,
        };
        (order, stem)
    });
    let mut examples: Vec<Example> = paths.iter().filter_map(|p| parse_example(p)).collect();

    fs::create_dir_all(&media_dir)?;

    // 渲染 + 拷贝媒体
    for ex in &mut examples {
        for sc in &mut ex.scenes {
            let mp4 = root.join(format!("out/{}.mp4", sc.name));
            if !skip_render && !mp4.exists() {
                print!("渲染 {}/{} ... ", ex.name, sc.name);
                use std::io::Write as _;
                std::io::stdout().flush().ok();
                let scene_arg = (!ex.single).then_some(sc.name.as_str());
                sc.rendered = render_scene(&ex.name, scene_arg);
                println!("{}", if sc.rendered { "ok" } else { "失败" });
            } else {
                sc.rendered = mp4.exists();
            }
            if sc.rendered {
                fs::copy(&mp4, media_dir.join(format!("{}.mp4", sc.name)))?;
            }
        }
    }

    // 生成画廊章节页 + SUMMARY（素材文件只出片拷媒体，不生成页面）
    fs::create_dir_all(&book_src)?;
    let mut chapters: Vec<(String, String)> = Vec::new();
    for ex in &examples {
        if let ChapterKind::Gallery(_, label) = chapter_kind(&ex.name) {
            fs::write(book_src.join(format!("{}.md", ex.name)), gen_chapter_md(ex))?;
            chapters.push((ex.name.clone(), label.to_string()));
        }
    }
    fs::write(book_src.join("SUMMARY.md"), gen_summary_md(&chapters))?;

    let total_rendered = examples
        .iter()
        .map(|e| e.scenes.iter().filter(|s| s.rendered).count())
        .sum::<usize>();
    let total: usize = examples.iter().map(|e| e.scenes.len()).sum();
    println!(
        "已生成 docs/book/src/（{total_rendered}/{total} 场景有视频），运行 `just book` 构建书籍"
    );
    Ok(())
}
