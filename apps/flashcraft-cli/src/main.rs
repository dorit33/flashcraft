//! flashcraft-cli — headless control of FlashCraft.
//!
//! Subcommands dispatch the same command ids the UI uses:
//! `drives.list`, `image.inspect`, `media.write`, `media.verify`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use std::io::Write as _;
use std::process::ExitCode;

use flashcraft_core::{self as core, command, registry, Error, WritePlan};
use flashcraft_native::linux;

/// Human progress on stderr; with `--progress` one JSON line per update on
/// stdout so the GUI (which may run us through pkexec) can render the bar.
fn progress_cb(machine: bool) -> impl FnMut(core::Progress) {
    let mut announced_sync = false;
    move |p: core::Progress| {
        if machine {
            if let Ok(line) = serde_json::to_string(&p) {
                println!("{line}");
                let _ = std::io::stdout().flush();
            }
        } else if p.phase == core::Phase::Syncing {
            if !announced_sync {
                announced_sync = true;
                eprintln!("\n  waiting for the device to flush (sync)…");
            }
        } else {
            let mib = p.written as f64 / 1_048_576.0;
            let speed = p.bytes_per_sec as f64 / 1_048_576.0;
            eprint!(
                "\r  {:>3}%  {:>9.1} MiB  {:>6.1} MiB/s   ",
                p.percent(),
                mib,
                speed
            );
            let _ = std::io::stderr().flush();
        }
    }
}

fn list_drives() -> Result<(), Error> {
    let drives = linux::list_drives_live()?;
    if drives.is_empty() {
        println!("no block devices found");
        return Ok(());
    }
    for d in &drives {
        let state = match d.writable() {
            Ok(()) => "writable".to_string(),
            Err(reason) => format!("not writable ({reason})"),
        };
        let gib = d.size_bytes as f64 / 1_073_741_824.0;
        println!(
            "{:<11} {:>7.1} GiB  removable={}  {:<20} {}",
            d.path,
            gib,
            d.removable,
            d.model.clone().unwrap_or_default(),
            state
        );
    }
    Ok(())
}

fn build_plan(image: &str, drive: &str) -> Result<WritePlan, Error> {
    let info = flashcraft_native::inspect_image(image)?;
    let drives = linux::list_drives_live()?;
    let dev = drives
        .iter()
        .find(|d| d.path == drive || d.name == drive)
        .cloned()
        .ok_or_else(|| Error::BadDrive(format!("no such device: {drive}")))?;
    WritePlan::new(info, dev)
}

fn usage() {
    println!(
        "flashcraft-cli — FlashCraft headless control

USAGE:
  flashcraft-cli <subcommand> [args]

SUBCOMMANDS:
  list                            List removable drives            (drives.list)
  inspect <image>                 Detect image kind and size       (image.inspect)
  write <image> <drive> [--yes]   Write image to drive             (media.write)
  verify <image> <drive>          Compare drive against image      (media.verify)

  --progress                      JSON progress lines on stdout (for the GUI)
  commands                        List every registered command

Drive may be a device node (/dev/sdb) or kernel name (sdb). Writing needs
root — run with sudo or pkexec."
    );
}

fn run(args: &[String]) -> Result<(), Error> {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "list" => list_drives(),
        "inspect" => {
            let image = args
                .get(1)
                .ok_or_else(|| Error::InvalidParams("usage: inspect <image>".into()))?;
            let info = flashcraft_native::inspect_image(image)?;
            println!("path:        {}", info.path);
            println!(
                "size:        {} bytes ({:.2} GiB)",
                info.size_bytes,
                info.size_bytes as f64 / 1_073_741_824.0
            );
            println!("kind:        {}", info.kind.name());
            if let Some(c) = &info.compression {
                println!("compression: {c}");
            }
            Ok(())
        }
        "write" => {
            let image = args.get(1).ok_or_else(|| {
                Error::InvalidParams("usage: write <image> <drive> [--yes]".into())
            })?;
            let drive = args.get(2).ok_or_else(|| {
                Error::InvalidParams("usage: write <image> <drive> [--yes]".into())
            })?;
            if linux::effective_uid() != Some(0) {
                return Err(Error::Refused(
                    "media.write needs root (sudo/pkexec flashcraft-cli write …)".into(),
                ));
            }
            let drives = linux::list_drives_live()?;
            let mut dev = drives
                .iter()
                .find(|d| d.path == *drive || d.name == *drive)
                .cloned()
                .ok_or_else(|| Error::BadDrive(format!("no such device: {drive}")))?;
            let spec = command("media.write");
            let destructive = spec.is_some_and(|s| s.destructive);
            if destructive && !args.iter().any(|a| a == "--yes") {
                return Err(Error::Refused(format!(
                    "media.write overwrites {} completely; repeat with --yes to confirm",
                    dev.path
                )));
            }
            let machine = args.iter().any(|a| a == "--progress");
            // Rufus parity: mounted partitions are unmounted after the
            // confirmation gate, then the plan is re-validated.
            if dev.writable() == Err("device has mounted partitions") {
                if machine {
                    println!("{{\"status\":\"unmounting\"}}");
                    let _ = std::io::stdout().flush();
                } else {
                    eprintln!(
                        "unmounting {} partition(s) on {} …",
                        dev.mounted.len(),
                        dev.path
                    );
                }
                linux::unmount_partitions(&mut dev)?;
            }
            let info = flashcraft_native::inspect_image(image)?;
            let plan = WritePlan::new(info, dev)?;
            eprintln!("writing {} -> {}", plan.image.path, plan.drive.path);
            flashcraft_native::write_image(&plan, &mut progress_cb(machine))?;
            eprintln!("\ndone; run `flashcraft-cli verify {image} {drive}` to check");
            Ok(())
        }
        "verify" => {
            let image = args
                .get(1)
                .ok_or_else(|| Error::InvalidParams("usage: verify <image> <drive>".into()))?;
            let drive = args
                .get(2)
                .ok_or_else(|| Error::InvalidParams("usage: verify <image> <drive>".into()))?;
            let plan = build_plan(image, drive)?;
            eprintln!("verifying {} against {}", plan.image.path, plan.drive.path);
            let machine = args.iter().any(|a| a == "--progress");
            flashcraft_native::verify_image(&plan, &mut progress_cb(machine))?;
            eprintln!("\nverify OK");
            Ok(())
        }
        "commands" => {
            for spec in registry() {
                println!(
                    "{:<14} {}{}",
                    spec.id,
                    if spec.destructive {
                        "[destructive] "
                    } else {
                        ""
                    },
                    spec.summary
                );
                println!("{:>15}params: {}", "", spec.params_doc);
            }
            Ok(())
        }
        "-h" | "--help" | "help" | "" => {
            usage();
            Ok(())
        }
        other => Err(Error::InvalidParams(format!(
            "unknown subcommand `{other}`; try --help"
        ))),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\nerror: {e}");
            ExitCode::FAILURE
        }
    }
}
