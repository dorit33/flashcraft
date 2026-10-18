//! cargo xtask — FlashCraft automation.
//!
//!   cargo xtask ci        fmt --check, clippy -D warnings, tests, wasm check
//!   cargo xtask layers    enforce the crate dependency layers
//!   cargo xtask wasm      cargo check L0–L2 for wasm32-unknown-unknown
//!   cargo xtask version [set X.Y.Z[-pre]]   print or set the workspace version

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

mod version;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".."))
}

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let cmd = if cmd == "cargo" {
        std::env::var("CARGO").unwrap_or_else(|_| cmd.to_string())
    } else {
        cmd.to_string()
    };
    let mut command = Command::new(&cmd);
    // Subcommands (cargo-fmt, cargo-clippy) resolve through PATH; make sure the
    // cargo binary's own directory is on it for the child.
    if let Some(dir) = std::path::Path::new(&cmd).parent() {
        let path = std::env::var("PATH").unwrap_or_default();
        command.env("PATH", format!("{}:{path}", dir.display()));
    }
    let status = command
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

fn version(args: &[String]) -> Result<(), String> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    version::run(&root(), &refs)
}

fn usage() {
    println!(
        "cargo xtask <command>

  ci        fmt --check, clippy -D warnings, tests, layers, wasm check
  layers    enforce crate dependency layers
  wasm      check core + ui-egui for wasm32-unknown-unknown
  version   print the workspace version, or `version set X.Y.Z[-pre]`"
    );
}

fn main() -> ExitCode {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let result = match arg.as_str() {
        "ci" => ci(),
        "layers" => layers(),
        "wasm" => wasm(),
        "version" => version(&std::env::args().skip(2).collect::<Vec<_>>()),
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
