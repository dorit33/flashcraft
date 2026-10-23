//! MCP server: expose the command registry over stdio JSON-RPC 2.0.
//!
//! `flashcraft-cli mcp` speaks the Model Context Protocol on stdin/stdout —
//! one newline-delimited JSON-RPC message per line. Tool names are the
//! registry ids with `_` for `.` (MCP tool names are `[a-zA-Z0-9_-]`).
//! Destructive tools keep the same structural gates as the CLI subcommands:
//! `confirm: true` in the arguments plus root privileges.

use std::io::{BufRead, Write as _};

use flashcraft_core::{self as core, registry, Error, FileSystem, PartitionScheme, WritePlan};
use flashcraft_native::linux;
use serde_json::{json, Value};

fn tool_name(id: &str) -> String {
    id.replace('.', "_")
}

fn send(v: &Value) {
    let mut out = std::io::stdout().lock();
    if let Ok(s) = serde_json::to_string(v) {
        let _ = writeln!(out, "{s}");
        let _ = out.flush();
    }
}

fn result(id: &Value, value: Value) {
    send(&json!({"jsonrpc": "2.0", "id": id, "result": value}));
}

fn rpc_error(id: &Value, code: i64, message: &str) {
    send(&json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}));
}

fn p_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn p_bool(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn p_u64(args: &Value, key: &str) -> u64 {
    args.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn tool_list() -> Value {
    let tools: Vec<Value> = registry()
        .iter()
        .map(|s| {
            json!({
                "name": tool_name(s.id),
                "description": format!(
                    "{} — {}{}",
                    s.label,
                    s.summary,
                    if s.destructive {
                        " [destructive: pass confirm:true and run as root]"
                    } else {
                        ""
                    }
                ),
                "inputSchema": serde_json::from_str::<Value>(s.input_schema)
                    .unwrap_or_else(|_| json!({"type": "object"})),
            })
        })
        .collect();
    json!({"tools": tools})
}

/// Progress as MCP `notifications/progress` when the caller passed a
/// `progressToken`; otherwise human lines on stderr (stdout is protocol-only).
fn mcp_progress(token: Option<Value>) -> impl FnMut(core::Progress) {
    move |p: core::Progress| {
        if let Some(t) = &token {
            send(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/progress",
                "params": {
                    "progressToken": t.clone(),
                    "progress": p.percent() as f64,
                    "total": 100.0,
                },
            }));
        } else {
            eprint!("\r  {:>3}%  {:?}   ", p.percent(), p.phase);
            let _ = std::io::stderr().flush();
        }
    }
}

fn need<'a>(args: &'a Value, key: &str, cmd: &str) -> Result<&'a str, Error> {
    p_str(args, key).ok_or_else(|| Error::InvalidParams(format!("{cmd} needs `{key}`")))
}

fn find_drive(drive: &str) -> Result<core::DriveInfo, Error> {
    linux::list_drives_live()?
        .into_iter()
        .find(|d| d.path == drive || d.name == drive)
        .ok_or_else(|| Error::BadDrive(format!("no such device: {drive}")))
}

fn call_tool(cmd: &str, args: &Value, token: Option<Value>) -> Result<String, Error> {
    match cmd {
        "drives.list" => {
            let out: Vec<Value> = linux::list_drives_live()?
                .iter()
                .map(|d| {
                    let mut v = serde_json::to_value(d).unwrap_or_else(|_| json!({}));
                    if let Some(o) = v.as_object_mut() {
                        o.insert(
                            "writable".to_string(),
                            match d.writable() {
                                Ok(()) => json!(true),
                                Err(reason) => json!({"refused": reason}),
                            },
                        );
                    }
                    v
                })
                .collect();
            serde_json::to_string_pretty(&json!({"drives": out}))
                .map_err(|e| Error::InvalidParams(e.to_string()))
        }
        "image.inspect" => {
            let info = flashcraft_native::inspect_image(need(args, "image", cmd)?)?;
            serde_json::to_string_pretty(&info).map_err(|e| Error::InvalidParams(e.to_string()))
        }
        "image.checksum" => {
            let sums =
                flashcraft_native::compute_checksums(need(args, "image", cmd)?, &mut |_| {})?;
            serde_json::to_string_pretty(&sums).map_err(|e| Error::InvalidParams(e.to_string()))
        }
        "media.write" => {
            let image = need(args, "image", cmd)?;
            let drive = need(args, "drive", cmd)?;
            if linux::effective_uid() != Some(0) {
                return Err(Error::Refused(
                    "media.write needs root — run the MCP server with sudo/pkexec".into(),
                ));
            }
            let mut dev = find_drive(drive)?;
            if !p_bool(args, "confirm") {
                return Err(Error::Refused(format!(
                    "media.write overwrites {} completely; pass confirm: true to proceed",
                    dev.path
                )));
            }
            if dev.writable() == Err("device has mounted partitions") {
                linux::unmount_partitions(&mut dev)?;
            }
            let info = flashcraft_native::inspect_image(image)?;
            let plan = WritePlan::new(info, dev)?
                .with_label(p_str(args, "label").map(str::to_string))
                .with_bad_blocks(p_u64(args, "bad_blocks") as u8)
                .with_persistence_mib(p_u64(args, "persistence_mib"))
                .with_old_bios_fixes(p_bool(args, "old_bios_fixes"))
                .with_verify_after(p_bool(args, "verify_after"))
                .with_extended_label(p_bool(args, "extended_label"));
            eprintln!("writing {} -> {}", plan.image.path, plan.drive.path);
            flashcraft_native::write_image(&plan, &mut mcp_progress(token))?;
            Ok(json!({
                "status": "written",
                "image": plan.image.path,
                "drive": plan.drive.path,
                "bytes": plan.image.size_bytes,
            })
            .to_string())
        }
        "media.verify" => {
            let image = need(args, "image", cmd)?;
            let drive = need(args, "drive", cmd)?;
            let plan = crate::build_plan(image, drive)?
                .with_label(p_str(args, "label").map(str::to_string))
                .with_old_bios_fixes(p_bool(args, "old_bios_fixes"));
            eprintln!("verifying {} against {}", plan.image.path, plan.drive.path);
            flashcraft_native::verify_image(&plan, &mut mcp_progress(token))?;
            Ok(json!({
                "status": "verified",
                "image": plan.image.path,
                "drive": plan.drive.path,
            })
            .to_string())
        }
        "media.format" => {
            let drive = need(args, "drive", cmd)?;
            if linux::effective_uid() != Some(0) {
                return Err(Error::Refused(
                    "media.format needs root — run the MCP server with sudo/pkexec".into(),
                ));
            }
            let mut dev = find_drive(drive)?;
            if !p_bool(args, "confirm") {
                return Err(Error::Refused(format!(
                    "media.format erases {} completely; pass confirm: true to proceed",
                    dev.path
                )));
            }
            if dev.writable() == Err("device has mounted partitions") {
                linux::unmount_partitions(&mut dev)?;
            }
            let scheme = p_str(args, "scheme")
                .map(PartitionScheme::parse)
                .unwrap_or(PartitionScheme::Mbr);
            let fs = p_str(args, "fs")
                .map(FileSystem::parse)
                .unwrap_or(FileSystem::Fat32);
            let cluster = p_u64(args, "cluster");
            let plan = flashcraft_core::FormatPlan::new(dev, scheme, fs)?
                .with_label(p_str(args, "label").map(str::to_string))
                .with_cluster_bytes(u32::try_from(cluster).unwrap_or(4096))
                .with_quick_format(!args.get("quick").is_some_and(|v| v == &Value::Bool(false)))
                .with_extended_label(p_bool(args, "extended_label"))
                .with_bad_blocks(p_u64(args, "bad_blocks") as u8)
                .with_old_bios_fixes(p_bool(args, "old_bios_fixes"));
            eprintln!(
                "formatting {} ({}, {})",
                plan.drive.path,
                plan.scheme.as_str(),
                plan.fs.as_str()
            );
            flashcraft_native::format_drive(&plan, &mut mcp_progress(token))?;
            Ok(json!({
                "status": "formatted",
                "drive": plan.drive.path,
                "scheme": plan.scheme.as_str(),
                "fs": plan.fs.as_str(),
            })
            .to_string())
        }
        "media.badblocks" => {
            let drive = need(args, "drive", cmd)?;
            let dev = find_drive(drive)?;
            let passes = p_u64(args, "passes").clamp(1, 4) as u8;
            flashcraft_native::scan_bad_blocks(&dev, passes, &mut mcp_progress(token))?;
            Ok(json!({
                "status": "scanned",
                "drive": dev.path,
                "passes": passes,
            })
            .to_string())
        }
        other => Err(Error::InvalidParams(format!("unknown tool `{other}`"))),
    }
}

fn dispatch(msg: &Value) {
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let id = msg.get("id").cloned();
    match method {
        "initialize" => {
            if let Some(i) = &id {
                let pv = msg
                    .pointer("/params/protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or("2024-11-05");
                result(
                    i,
                    json!({
                        "protocolVersion": pv,
                        "capabilities": {"tools": {}},
                        "serverInfo": {
                            "name": "flashcraft",
                            "version": env!("CARGO_PKG_VERSION"),
                        },
                    }),
                );
            }
        }
        "ping" => {
            if let Some(i) = &id {
                result(i, json!({}));
            }
        }
        "tools/list" => {
            if let Some(i) = &id {
                result(i, tool_list());
            }
        }
        "tools/call" => {
            if let Some(i) = &id {
                let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let token = params.pointer("/_meta/progressToken").cloned();
                match call_tool(&name.replace('_', "."), &args, token) {
                    Ok(text) => result(i, json!({"content": [{"type": "text", "text": text}]})),
                    Err(e) => result(
                        i,
                        json!({"content": [{"type": "text", "text": format!("error: {e}")}],
                               "isError": true}),
                    ),
                }
            }
        }
        "resources/list" => {
            if let Some(i) = &id {
                result(i, json!({"resources": []}));
            }
        }
        "resources/templates/list" => {
            if let Some(i) = &id {
                result(i, json!({"resourceTemplates": []}));
            }
        }
        "prompts/list" => {
            if let Some(i) = &id {
                result(i, json!({"prompts": []}));
            }
        }
        m if m.starts_with("notifications/") || m.starts_with("logging/") => {}
        m if m.starts_with("completion/") || m.starts_with("roots/") => {
            if let Some(i) = &id {
                result(i, json!({}));
            }
        }
        _ => {
            if let Some(i) = &id {
                rpc_error(i, -32601, &format!("method not found: {method}"));
            }
        }
    }
}

/// Serve MCP over stdio until stdin closes.
pub fn serve() -> Result<(), Error> {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(msg) => dispatch(&msg),
            Err(e) => rpc_error(&Value::Null, -32700, &format!("parse error: {e}")),
        }
    }
    Ok(())
}
