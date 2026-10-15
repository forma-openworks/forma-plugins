//! Сборка плагинов Forma.
//!
//! - `cargo xtask build <плагин>` — собирает плагин в `target/fpl/<плагин>`: манифест, данные
//!   и модуль WebAssembly (для плагинов с code — `cargo build --target wasm32-wasip2`). Эту
//!   папку можно подключить в программе в режиме разработчика.
//! - `cargo xtask pack <плагин>` — то же и упаковывает в `dist/<id>.fpl`.
//! - `cargo xtask pack all` — все плагины из `plugins/`.
//!
//! Лишние аргументы после имени плагина уходят в `cargo build` (например,
//! `--config <файл>` с локальным Forma Core).

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(name)) = (args.first(), args.get(1)) else {
        eprintln!("usage: cargo xtask build|pack <plugin>|all [cargo args…]");
        std::process::exit(2);
    };
    let extra = &args[2..];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask лежит в корне").to_path_buf();
    let names: Vec<String> = if name == "all" {
        let mut v: Vec<String> = std::fs::read_dir(root.join("plugins")).expect("plugins").flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    } else {
        vec![name.clone()]
    };
    for name in names {
        let result = build(&root, &name, extra).and_then(|staged| match cmd.as_str() {
            "build" => {
                println!("{name}: {}", staged.display());
                Ok(())
            }
            "pack" => {
                let manifest = forma_plugins::read_manifest(&staged)?;
                let dist = root.join("dist");
                std::fs::create_dir_all(&dist).map_err(|e| e.to_string())?;
                let out = dist.join(format!("{}.fpl", manifest.plugin.id));
                forma_plugins::pack(&staged, &out)?;
                println!("{name}: {}", out.display());
                Ok(())
            }
            other => Err(format!("unknown command {other:?}")),
        });
        if let Err(e) = result {
            eprintln!("{name}: {e}");
            std::process::exit(1);
        }
    }
}

/// Собирает плагин в `target/fpl/<name>` и возвращает эту папку.
fn build(root: &Path, name: &str, extra: &[String]) -> Result<PathBuf, String> {
    let src = root.join("plugins").join(name);
    let manifest = forma_plugins::read_manifest(&src)?;
    let staged = root.join("target").join("fpl").join(name);
    let _ = std::fs::remove_dir_all(&staged);
    std::fs::create_dir_all(&staged).map_err(|e| e.to_string())?;
    // Всё из папки плагина, кроме исходников Rust.
    for e in std::fs::read_dir(&src).map_err(|e| e.to_string())?.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if matches!(n.as_str(), "src" | "Cargo.toml" | "target") || n.starts_with('.') {
            continue;
        }
        copy(&e.path(), &staged.join(&n))?;
    }
    if let Some(code) = &manifest.code {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .current_dir(root)
            .args(["build", "--release", "--target", "wasm32-wasip2", "-p", name])
            .args(extra)
            .status()
            .map_err(|e| format!("cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build failed".into());
        }
        let wasm = root.join("target").join("wasm32-wasip2").join("release").join(format!("{}.wasm", name.replace('-', "_")));
        std::fs::copy(&wasm, staged.join(&code.wasm)).map_err(|e| format!("{}: {e}", wasm.display()))?;
    }
    Ok(staged)
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    if from.is_dir() {
        std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
        for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
            copy(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("{}: {e}", from.display()))
    }
}
