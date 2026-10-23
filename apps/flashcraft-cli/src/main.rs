//! flashcraft-cli — headless control of FlashCraft.
//!
//! Subcommands dispatch the same command ids the UI uses:
//! `drives.list`, `image.inspect`, `image.checksum`, `media.write`,
//! `media.verify`, `media.format`, `media.badblocks`. The `mcp`
//! subcommand serves the same registry over MCP (stdio JSON-RPC).

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

use flashcraft_core::{
    self as core, command, registry, Error, FileSystem, FormatPlan, PartitionScheme, WritePlan,
};
use flashcraft_native::linux;

mod mcp;

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

fn flag_val(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

fn flag_u64(args: &[String], flag: &str) -> u64 {
    flag_val(args, flag)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn usage() {
    println!(
        "flashcraft-cli — FlashCraft headless control

USAGE:
  flashcraft-cli <subcommand> [args]

SUBCOMMANDS:
  list                            List removable drives            (drives.list)
  inspect <image>                 Detect image kind and size       (image.inspect)
  checksum <image>                Compute MD5, SHA-1, SHA-256      (image.checksum)
  write <image> <drive> [--yes]   Write image to drive             (media.write)
  verify <image> <drive>          Compare drive against image      (media.verify)
  format <drive> [--yes]          Partition & format drive         (media.format)
  badblocks <drive>               Scan drive for bad blocks        (media.badblocks)
  commands                        List every registered command
  mcp                             Serve the registry over MCP (stdio JSON-RPC)

OPTIONS:
  --progress                      JSON progress lines on stdout (for the GUI)
  --label TEXT                    Volume label for ISO / formatted partition
  --scheme MBR|GPT                Partition scheme for format (default: MBR)
  --fs FS                         FAT32|NTFS|exFAT|UDF|ext2|ext3|ext4
  --cluster BYTES                 Cluster size in bytes (default: 4096)
  --no-quick                      Full zero-fill format instead of quick format
  --extended-label                Write autorun.inf extended label files
  --bad-blocks PASSES             Scan for bad blocks (1..4 passes)
  --persistence MIB               Append persistent partition of MIB megabytes
  --old-bios-fixes                Apply legacy BIOS MBR active-flag fix
  --verify                        Verify written bytes after writing

Drive may be a device node (/dev/sdb) or kernel name (sdb). Writing needs
root — run with sudo or pkexec."
    );
}

fn unmount_if_needed(dev: &mut core::DriveInfo, machine: bool) -> Result<(), Error> {
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
        linux::unmount_partitions(dev)?;
    }
    Ok(())
}

fn run(args: &[String]) -> Result<(), Error> {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let machine = has_flag(args, "--progress");
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
        "checksum" => {
            let image = args
                .get(1)
                .ok_or_else(|| Error::InvalidParams("usage: checksum <image>".into()))?;
            let sums = flashcraft_native::compute_checksums(image, &mut progress_cb(machine))?;
            if !machine {
                eprintln!();
            }
            println!("MD5:     {}", sums.md5);
            println!("SHA-1:   {}", sums.sha1);
            println!("SHA-256: {}", sums.sha256);
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
            if destructive && !has_flag(args, "--yes") {
                return Err(Error::Refused(format!(
                    "media.write overwrites {} completely; repeat with --yes to confirm",
                    dev.path
                )));
            }
            unmount_if_needed(&mut dev, machine)?;
            let info = flashcraft_native::inspect_image(image)?;
            let plan = WritePlan::new(info, dev)?
                .with_label(flag_val(args, "--label"))
                .with_bad_blocks(flag_u64(args, "--bad-blocks") as u8)
                .with_persistence_mib(flag_u64(args, "--persistence"))
                .with_old_bios_fixes(has_flag(args, "--old-bios-fixes"))
                .with_verify_after(has_flag(args, "--verify"))
                .with_extended_label(has_flag(args, "--extended-label"));
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
            let plan = build_plan(image, drive)?
                .with_label(flag_val(args, "--label"))
                .with_old_bios_fixes(has_flag(args, "--old-bios-fixes"));
            eprintln!("verifying {} against {}", plan.image.path, plan.drive.path);
            flashcraft_native::verify_image(&plan, &mut progress_cb(machine))?;
            eprintln!("\nverify OK");
            Ok(())
        }
        "format" => {
            let drive = args
                .get(1)
                .ok_or_else(|| Error::InvalidParams("usage: format <drive> [--yes]".into()))?;
            if linux::effective_uid() != Some(0) {
                return Err(Error::Refused(
                    "media.format needs root (sudo/pkexec flashcraft-cli format …)".into(),
                ));
            }
            let drives = linux::list_drives_live()?;
            let mut dev = drives
                .iter()
                .find(|d| d.path == *drive || d.name == *drive)
                .cloned()
                .ok_or_else(|| Error::BadDrive(format!("no such device: {drive}")))?;
            if !has_flag(args, "--yes") {
                return Err(Error::Refused(format!(
                    "media.format erases {} completely; repeat with --yes to confirm",
                    dev.path
                )));
            }
            unmount_if_needed(&mut dev, machine)?;
            let scheme = flag_val(args, "--scheme")
                .as_deref()
                .map(PartitionScheme::parse)
                .unwrap_or(PartitionScheme::Mbr);
            let fs = flag_val(args, "--fs")
                .as_deref()
                .map(FileSystem::parse)
                .unwrap_or(FileSystem::Fat32);
            let cluster = flag_val(args, "--cluster")
                .as_deref()
                .map(core::parse_cluster_bytes)
                .unwrap_or(4096);
            let plan = FormatPlan::new(dev, scheme, fs)?
                .with_label(flag_val(args, "--label"))
                .with_cluster_bytes(cluster)
                .with_quick_format(!has_flag(args, "--no-quick"))
                .with_extended_label(has_flag(args, "--extended-label"))
                .with_bad_blocks(flag_u64(args, "--bad-blocks") as u8)
                .with_old_bios_fixes(has_flag(args, "--old-bios-fixes"));
            eprintln!(
                "formatting {} ({}, {})",
                plan.drive.path,
                plan.scheme.as_str(),
                plan.fs.as_str()
            );
            flashcraft_native::format_drive(&plan, &mut progress_cb(machine))?;
            eprintln!("\nformat done");
            Ok(())
        }
        "badblocks" => {
            let drive = args
                .get(1)
                .ok_or_else(|| Error::InvalidParams("usage: badblocks <drive>".into()))?;
            let drives = linux::list_drives_live()?;
            let dev = drives
                .iter()
                .find(|d| d.path == *drive || d.name == *drive)
                .cloned()
                .ok_or_else(|| Error::BadDrive(format!("no such device: {drive}")))?;
            let passes = flag_u64(args, "--passes").clamp(1, 4) as u8;
            flashcraft_native::scan_bad_blocks(&dev, passes, &mut progress_cb(machine))?;
            eprintln!("\nbad-block scan OK");
            Ok(())
        }
        "commands" => {
            for spec in registry() {
                println!(
                    "{:<16} {}{}",
                    spec.id,
                    if spec.destructive {
                        "[destructive] "
                    } else {
                        ""
                    },
                    spec.summary
                );
                println!("{:>17}params: {}", "", spec.params_doc);
            }
            Ok(())
        }
        "mcp" => mcp::serve(),
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
