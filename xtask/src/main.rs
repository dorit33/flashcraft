//! cargo xtask — FlashCraft automation.
//!
//!   cargo xtask ci        fmt --check, clippy -D warnings, tests, wasm check
//!   cargo xtask layers    enforce the crate dependency layers
//!   cargo xtask wasm      cargo check L0–L2 for wasm32-unknown-unknown
//!   cargo xtask version   print the workspace version

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".."))
}

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|e| format!("cannot run {cmd}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{cmd} {} failed", args.join(" ")))
    }
}

/// Layer map: a crate may only depend on crates on lower layers.
/// (crate dir name, layer, forbidden workspace deps)
const LAYERS: &[(&str, &str, &[&str])] = &[
    (
        "crates/core",
        "L0",
        &["flashcraft-native", "flashcraft-ui-egui"],
    ),
    ("crates/native", "L1", &["flashcraft-ui-egui"]),
    ("crates/ui-egui", "L2", &[]),
    ("apps/flashcraft", "L3", &[]),
    ("apps/flashcraft-cli", "L3", &["flashcraft-ui-egui"]),
];

fn layers() -> Result<(), String> {
    for (dir, layer, forbidden) in LAYERS {
        let manifest = root().join(dir).join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest)
            .map_err(|e| format!("cannot read {}: {e}", manifest.display()))?;
        for dep in *forbidden {
            if text.contains(&format!("{dep} =")) || text.contains(&format!("{dep}=")) {
                return Err(format!("{dir} ({layer}) must not depend on {dep}"));
            }
        }
    }
    println!("layers OK");
    Ok(())
}

fn wasm() -> Result<(), String> {
    run(
        "cargo",
        &[
            "check",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            "flashcraft-core",
            "-p",
            "flashcraft-ui-egui",
        ],
    )
}

fn ci() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace"])?;
    layers()?;
    wasm()
}

fn version() -> Result<(), String> {
    let manifest = root().join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .map_err(|e| format!("cannot read workspace manifest: {e}"))?;
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("version = ") {
            println!("{}", v.trim_matches('"'));
            return Ok(());
        }
    }
    Err("workspace version not found".to_string())
}

fn usage() {
    println!(
        "cargo xtask <command>

  ci        fmt --check, clippy -D warnings, tests, layers, wasm check
  layers    enforce crate dependency layers
  wasm      check core + ui-egui for wasm32-unknown-unknown
  version   print the workspace version"
    );
}

fn main() -> ExitCode {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let result = match arg.as_str() {
        "ci" => ci(),
        "layers" => layers(),
        "wasm" => wasm(),
        "version" => version(),
        _ => {
            usage();
            if arg.is_empty() {
                Ok(())
            } else {
                Err(format!("unknown command: {arg}"))
            }
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
