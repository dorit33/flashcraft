//! `cargo xtask version [set X.Y.Z[-pre]]`: the workspace version, the single source of truth.
//!
//! The version lives in `[workspace.package] version` in the root `Cargo.toml`; every crate
//! inherits it with `version.workspace = true`, and the packaging scripts read it from here.
//! `set` also stamps the RPM spec's `Version:` header and the AppStream `<release>` entry so
//! package metadata can never drift from the crate version.

use std::path::Path;

/// The `version = "…"` value inside `[workspace.package]`.
pub fn read(manifest: &str) -> Result<String, String> {
    let (_, line) = find_line(manifest)?;
    let v = line
        .split_once('=')
        .map(|(_, v)| v.trim().trim_matches('"').to_string())
        .unwrap_or_default();
    if v.is_empty() {
        return Err("empty [workspace.package] version".into());
    }
    Ok(v)
}

/// `manifest` with the workspace version replaced; everything else is byte-for-byte unchanged.
pub fn replace(manifest: &str, new: &str) -> Result<String, String> {
    validate(new)?;
    let (idx, _) = find_line(manifest)?;
    let mut out = String::with_capacity(manifest.len() + 8);
    for (i, line) in manifest.split_inclusive('\n').enumerate() {
        if i == idx {
            let eol = if line.ends_with("\r\n") { "\r\n" } else { "\n" };
            out.push_str(&format!("version = \"{new}\"{eol}"));
        } else {
            out.push_str(line);
        }
    }
    Ok(out)
}

/// Semver without build metadata: `MAJOR.MINOR.PATCH` plus an optional `-pre.release` tag.
pub fn validate(v: &str) -> Result<(), String> {
    let bad = || Err(format!("`{v}` is not a version like 1.2.3 or 1.2.3-rc.1"));
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let nums: Vec<&str> = core.split('.').collect();
    if nums.len() != 3
        || nums.iter().any(|n| {
            n.is_empty()
                || !n.bytes().all(|b| b.is_ascii_digit())
                || (n.len() > 1 && n.starts_with('0'))
        })
    {
        return bad();
    }
    if let Some(p) = pre {
        if p.is_empty()
            || p.split('.').any(|id| {
                id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return bad();
        }
    }
    Ok(())
}

/// Index and text of the `version` line in `[workspace.package]`.
fn find_line(manifest: &str) -> Result<(usize, &str), String> {
    let mut in_section = false;
    for (i, line) in manifest.lines().enumerate() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = t == "[workspace.package]";
            continue;
        }
        if in_section {
            if let Some(rest) = t.strip_prefix("version") {
                if rest.trim_start().starts_with('=') {
                    return Ok((i, line));
                }
            }
        }
    }
    Err("no `version = \"…\"` in [workspace.package] of the root Cargo.toml".into())
}

/// `path` written atomically via a sibling temp file.
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let tmp = path.with_extension("xtask-tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("replace {}: {e}", path.display()))
}

/// Rewrite the RPM spec's `Version:` header to `new`.
fn stamp_spec(root: &Path, new: &str) -> Result<(), String> {
    let path = root.join("packaging/linux/rpm/flashcraft.spec");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut out = String::with_capacity(text.len() + 8);
    let mut done = false;
    for line in text.split_inclusive('\n') {
        if !done && line.starts_with("Version:") {
            let eol = line
                .strip_prefix("Version:")
                .and_then(|r| r.find('\n').map(|i| &r[i..]))
                .unwrap_or("\n");
            out.push_str(&format!("Version:        {new}{eol}"));
            done = true;
        } else {
            out.push_str(line);
        }
    }
    if !done {
        return Err("no `Version:` header in flashcraft.spec".into());
    }
    write_atomic(&path, &out)
}

/// Rewrite the AppStream `<release version="…" date="…"/>` entry to `new` + today.
fn stamp_metainfo(root: &Path, new: &str) -> Result<(), String> {
    let path = root.join("packaging/linux/dev.flashcraft.app.metainfo.xml");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut out = String::with_capacity(text.len() + 32);
    let mut done = false;
    for line in text.split_inclusive('\n') {
        if !done && line.contains("<release version=\"") {
            let eol = if line.ends_with("\r\n") { "\r\n" } else { "\n" };
            out.push_str(&format!(
                "    <release version=\"{new}\" date=\"{}\"/>{eol}",
                today()
            ));
            done = true;
        } else {
            out.push_str(line);
        }
    }
    if !done {
        return Err("no `<release version=\"…\">` in metainfo.xml".into());
    }
    write_atomic(&path, &out)
}

/// Today's date as YYYY-MM-DD, from `SystemTime` — no chrono dependency.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    // Civil-from-days (Howard Hinnant's algorithm), constants shifted for the
    // proleptic Gregorian calendar.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn run(root: &Path, args: &[&str]) -> Result<(), String> {
    let path = root.join("Cargo.toml");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    match args {
        [] => {
            println!("{}", read(&text)?);
            Ok(())
        }
        ["set", new] => {
            let old = read(&text)?;
            if old == *new {
                println!("version: {old} (unchanged)");
                return Ok(());
            }
            let updated = replace(&text, new)?;
            write_atomic(&path, &updated)?;
            stamp_spec(root, new)?;
            stamp_metainfo(root, new)?;
            // Refresh the workspace members' entries in Cargo.lock (no dependency upgrades).
            if crate::run("cargo", &["update", "--workspace", "--offline"]).is_err() {
                crate::run("cargo", &["update", "--workspace"])?;
            }
            println!("version: {old} -> {new}");
            Ok(())
        }
        _ => Err("usage: cargo xtask version [set X.Y.Z[-pre]]".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str =
        "[workspace]\nmembers = [\"a\"]\n\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace.dependencies]\nfoo = { version = \"1\" }\n";

    #[test]
    fn reads_the_workspace_version() {
        assert_eq!(read(MANIFEST).unwrap_or_default(), "0.1.0");
        assert!(read("[package]\nversion = \"1.0.0\"\n").is_err());
    }

    #[test]
    fn replaces_only_the_workspace_version() {
        let out = replace(MANIFEST, "1.2.3-rc.1").unwrap_or_default();
        assert_eq!(read(&out).unwrap_or_default(), "1.2.3-rc.1");
        assert_eq!(
            out,
            MANIFEST.replace("version = \"0.1.0\"", "version = \"1.2.3-rc.1\"")
        );
        assert!(out.contains("foo = { version = \"1\" }"));
    }

    #[test]
    fn keeps_crlf_line_endings() {
        let crlf = MANIFEST.replace('\n', "\r\n");
        let out = replace(&crlf, "2.0.0").unwrap_or_default();
        assert_eq!(out, crlf.replace("\"0.1.0\"", "\"2.0.0\""));
    }

    #[test]
    fn validates_versions() {
        for ok in [
            "0.1.0",
            "1.20.300",
            "1.0.0-rc.1",
            "1.0.0-alpha",
            "1.0.0-x-y.2",
        ] {
            assert!(validate(ok).is_ok(), "{ok}");
        }
        for bad in [
            "1.0",
            "1.0.0.0",
            "v1.0.0",
            "01.0.0",
            "1.0.0-",
            "1.0.0-a..b",
            "1.0.0+meta",
            "1.a.0",
            "",
        ] {
            assert!(validate(bad).is_err(), "{bad}");
        }
        assert!(replace(MANIFEST, "nope").is_err());
    }

    #[test]
    fn the_real_manifest_has_a_valid_version() {
        let text = std::fs::read_to_string(crate::root().join("Cargo.toml")).unwrap_or_default();
        assert!(validate(&read(&text).unwrap_or_default()).is_ok());
    }
}
