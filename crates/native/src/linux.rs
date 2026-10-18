//! Linux device enumeration from sysfs. Everything is read-only text
//! parsing; the `sysfs_root` parameter lets tests run against a fixture
//! instead of the live system.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use flashcraft_core::{DriveInfo, Error, Result};

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_u64(path: &Path) -> Option<u64> {
    read_trimmed(path).and_then(|s| s.parse::<u64>().ok())
}

/// Device nodes currently mounted anywhere, from `/proc/mounts`-style text.
pub fn mounted_devices(mounts_text: &str) -> BTreeSet<String> {
    mounts_text
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|dev| dev.starts_with("/dev/"))
        .map(|dev| dev.to_string())
        .collect()
}

/// Enumerate block devices under `sysfs_root` (normally `/sys/block`).
/// `mounts_text` is `/proc/mounts` content so tests can supply fixtures.
pub fn list_drives(sysfs_root: &Path, mounts_text: &str) -> Result<Vec<DriveInfo>> {
    let mounted = mounted_devices(mounts_text);
    let entries = fs::read_dir(sysfs_root)
        .map_err(|e| Error::Backend(format!("cannot read {}: {e}", sysfs_root.display())))?;

    let mut drives = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip virtual devices: loop, ram, dm, zram.
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("dm-")
            || name.starts_with("zram")
        {
            continue;
        }
        let sectors = match read_u64(&dir.join("size")) {
            Some(s) => s,
            None => continue,
        };
        let removable = read_u64(&dir.join("removable")) == Some(1);
        let dev_path = PathBuf::from("/dev").join(&name);
        // A device counts as mounted if it or any `name`-prefixed partition is.
        let dev_str = dev_path.to_string_lossy().to_string();
        let mounted_here: Vec<String> = mounted
            .iter()
            .filter(|m| m.starts_with(&dev_str))
            .cloned()
            .collect();
        drives.push(DriveInfo {
            name,
            path: dev_str,
            size_bytes: sectors.saturating_mul(512),
            removable,
            vendor: read_trimmed(&dir.join("device/vendor")),
            model: read_trimmed(&dir.join("device/model")),
            mounted: mounted_here,
        });
    }
    drives.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(drives)
}

/// Live enumeration: `/sys/block` + `/proc/mounts`.
pub fn list_drives_live() -> Result<Vec<DriveInfo>> {
    let mounts = fs::read_to_string("/proc/mounts")
        .map_err(|e| Error::Backend(format!("cannot read /proc/mounts: {e}")))?;
    list_drives(Path::new("/sys/block"), &mounts)
}

fn is_mounted(dev: &str) -> bool {
    fs::read_to_string("/proc/mounts")
        .map(|m| mounted_devices(&m).contains(dev))
        .unwrap_or(true)
}

fn try_unmount(prog: &str, args: &[&str]) -> std::result::Result<(), String> {
    match std::process::Command::new(prog).args(args).output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Unmount every mounted partition listed on `drive` — the Rufus behaviour:
/// START unmounts the volumes itself instead of refusing. Unprivileged runs
/// go through `udisksctl` (polkit handles user-mounted volumes); elevated
/// runs skip it — under pkexec there is no user session for udisks to consult,
/// so root goes straight to `umount` with a lazy `umount -l` last resort for
/// busy mounts (the Rufus "force dismount" equivalent). On success
/// `drive.mounted` is emptied; anything still mounted is an error naming the
/// tools' own diagnostics.
pub fn unmount_partitions(drive: &mut DriveInfo) -> Result<()> {
    let elevated = effective_uid() == Some(0);
    for dev in drive.mounted.clone() {
        let attempts: Vec<(&str, Vec<String>)> = if elevated {
            vec![
                ("umount", vec![dev.clone()]),
                ("umount", vec!["-l".into(), dev.clone()]),
            ]
        } else {
            vec![
                (
                    "udisksctl",
                    vec!["unmount".into(), "-b".into(), dev.clone()],
                ),
                ("umount", vec![dev.clone()]),
                ("umount", vec!["-l".into(), dev.clone()]),
            ]
        };
        let mut errs = Vec::new();
        let mut gone = false;
        for (prog, args) in &attempts {
            let argrefs: Vec<&str> = args.iter().map(String::as_str).collect();
            match try_unmount(prog, &argrefs) {
                Ok(()) => gone = true,
                Err(e) => errs.push(format!("{prog}: {e}")),
            }
            if gone || !is_mounted(&dev) {
                gone = true;
                break;
            }
        }
        if !gone {
            return Err(Error::Backend(format!(
                "cannot unmount {dev} ({})",
                errs.join("; ")
            )));
        }
    }
    // Trust but verify: re-read /proc/mounts rather than believing exit codes.
    let mounts = fs::read_to_string("/proc/mounts")
        .map_err(|e| Error::Backend(format!("cannot read /proc/mounts: {e}")))?;
    let still = mounted_devices(&mounts);
    drive.mounted.retain(|m| still.contains(m));
    if drive.mounted.is_empty() {
        Ok(())
    } else {
        Err(Error::Backend(format!(
            "still mounted: {}",
            drive.mounted.join(", ")
        )))
    }
}

/// Effective uid without libc/unsafe: parse `/proc/self/status`.
pub fn effective_uid() -> Option<u32> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|l| l.starts_with("Uid:"))
        .and_then(|l| l.split_whitespace().nth(2))
        .and_then(|s| s.parse::<u32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_parse_only_dev_entries() {
        let text = "proc /proc proc rw 0 0\n/dev/sdb1 /run/media/u/x ext4 rw 0 0\ntmpfs /tmp tmpfs rw 0 0\n";
        let m = mounted_devices(text);
        assert!(m.contains("/dev/sdb1"));
        assert!(!m.contains("tmpfs"));
    }

    #[test]
    fn lists_fixture_drives() {
        let root = std::env::temp_dir().join(format!("flashcraft-test-{}", std::process::id()));
        let sda = root.join("sda");
        let sdb = root.join("sdb");
        let _ = fs::create_dir_all(&sda);
        let _ = fs::create_dir_all(&sdb);
        let _ = fs::write(sda.join("size"), "1000");
        let _ = fs::write(sda.join("removable"), "0");
        let _ = fs::write(sdb.join("size"), "2000");
        let _ = fs::write(sdb.join("removable"), "1");
        let _ = fs::write(sdb.join("device"), "");

        let drives = list_drives(&root, "/dev/sdb1 /mnt ext4 rw 0 0\n").unwrap();
        assert_eq!(drives.len(), 2);
        let sdbi = drives.iter().find(|d| d.name == "sdb");
        assert!(sdbi.is_some_and(|d| d.removable && d.size_bytes == 2000 * 512));
        assert!(sdbi.is_some_and(|d| d.mounted == vec!["/dev/sdb1".to_string()]));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unmount_empty_is_noop() {
        let mut d = DriveInfo {
            name: "sdb".into(),
            path: "/dev/sdb".into(),
            size_bytes: 1024,
            removable: true,
            vendor: None,
            model: None,
            mounted: vec![],
        };
        unmount_partitions(&mut d).unwrap();
        assert!(d.mounted.is_empty());
    }

    #[test]
    fn unmount_stale_entry_is_noop() {
        // A node listed but not actually mounted in /proc/mounts is treated
        // as already gone — unmount must not fail on stale state.
        let mut d = DriveInfo {
            name: "sdb".into(),
            path: "/dev/sdb".into(),
            size_bytes: 1024,
            removable: true,
            vendor: None,
            model: None,
            mounted: vec!["/dev/fc-nonexistent-node".into()],
        };
        unmount_partitions(&mut d).unwrap();
        assert!(d.mounted.is_empty());
    }

    #[test]
    fn skips_virtual_devices() {
        let root = std::env::temp_dir().join(format!("flashcraft-test2-{}", std::process::id()));
        for name in ["loop0", "ram0", "dm-0", "zram0", "sdb"] {
            let d = root.join(name);
            let _ = fs::create_dir_all(&d);
            let _ = fs::write(d.join("size"), "10");
            let _ = fs::write(d.join("removable"), "1");
        }
        let drives = list_drives(&root, "").unwrap();
        assert_eq!(drives.len(), 1);
        assert_eq!(drives[0].name, "sdb");
        let _ = fs::remove_dir_all(&root);
    }
}
