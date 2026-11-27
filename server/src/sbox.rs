//! the only sbox-b client there is. `create` takes the image an agent's
//! ## Why a hand-rolled client

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use angela::sandbox::SandboxRow;

/// Where sbox-b listens. Overridable because the port is sbox-b's choice,
fn base() -> (String, u16) {
    let host = std::env::var("SBOX_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("SBOX_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000u16);
    (host, port)
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const READ_TIMEOUT: Duration = Duration::from_secs(180);

fn request(method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    let (host, port) = base();
    let addr = format!("{host}:{port}");
    let sock = addr
        .parse()
        .map_err(|_| format!("bad sbox address: {addr}"))
        .and_then(|a| {
            TcpStream::connect_timeout(&a, CONNECT_TIMEOUT)
                .map_err(|e| format!("sbox is not reachable at {addr}: {e}"))
        })?;
    sock.set_read_timeout(Some(if method == "GET" { CONNECT_TIMEOUT } else { READ_TIMEOUT })).ok();
    sock.set_write_timeout(Some(CONNECT_TIMEOUT)).ok();
    let mut sock = sock;

    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: application/json\r\n"
    );
    if body.is_some() {
        head.push_str("Content-Type: application/json\r\n");
        head.push_str(&format!("Content-Length: {}\r\n", payload.len()));
    }
    head.push_str("\r\n");
    sock.write_all(head.as_bytes()).map_err(|e| e.to_string())?;
    if !payload.is_empty() {
        sock.write_all(payload.as_bytes()).map_err(|e| e.to_string())?;
    }
    sock.flush().ok();

    let mut raw = Vec::new();
    sock.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let Some(split) = text.find("\r\n\r\n") else {
        return Err("sbox sent a malformed response".into());
    };
    let (headers, rest) = text.split_at(split);
    let status: u16 = headers
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body_text = rest.trim_start_matches("\r\n\r\n").trim();
    let parsed: Value = serde_json::from_str(body_text).unwrap_or(Value::Null);

    if (200..300).contains(&status) {
        return Ok(parsed);
    }
    // inventing our own, because it is the half that knows why.
    let msg = parsed
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| {
            parsed
                .get("error")
                .and_then(|e| e.as_object())
                .and_then(|o| o.get("message"))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
        .unwrap_or_else(|| format!("sbox returned HTTP {status}"));
    Err(msg)
}

/// (this box is a Blackwell 5090) cuBLAS must PTX-JIT — which needs the
/// middle: exactly the driver libs, mounted read-only at /usr/local/lib.
const GPU_DRIVER_LIB_PREFIXES: &[&str] = &[
    "libcuda.so.",
    "libnvidia-nvvm",
    "libnvidia-ptxjitcompiler.so.",
    "libnvidia-ml.so.",
    "libnvidia-allocator.so.",
];

const MAX_GPU_LIBS: usize = 8;

fn is_soname(file: &str) -> bool {
    let Some((_, tail)) = file.split_once(".so.") else { return false };
    !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_sonames_pass_the_driver_lib_filter() {
        assert!(is_soname("libcuda.so.1"));
        assert!(is_soname("libnvidia-nvvm70.so.4"));
        assert!(!is_soname("libcuda.so"));
        assert!(!is_soname("libcuda.so.615.78.08"));
        assert!(!is_soname("libnvidia-eglcore.so.615.78.08"));
    }
}

/// knows one — it carries the driver libraries with it. Otherwise pass the raw
/// `/dev/nvidia*` nodes — only those this process can actually open, because
fn gpu_setup() -> (Vec<String>, Vec<(String, String)>) {
    for dir in ["/etc/cdi", "/usr/share/cdi", "/var/run/cdi"] {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if name.contains("nvidia") {
                    return (vec!["nvidia.com/gpu".to_string()], Vec::new());
                }
            }
        }
    }
    let mut devices = Vec::new();
    if let Ok(rd) = std::fs::read_dir("/dev") {
        let mut names: Vec<String> = rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("nvidia"))
            .collect();
        names.sort();
        for n in names {
            let p = format!("/dev/{n}");
            // The node must be usable by this user; rootless podman passes it
            // through only if it is.
            if std::fs::OpenOptions::new().read(true).write(true).open(&p).is_ok() {
                devices.push(p);
            }
        }
    }
    if devices.is_empty() {
        return (devices, Vec::new());
    }
    let mut libs: Vec<(String, String)> = Vec::new();
    for dir in ["/usr/lib", "/usr/lib64", "/usr/lib/x86_64-linux-gnu"] {
        if libs.len() >= MAX_GPU_LIBS {
            break;
        }
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut names: Vec<String> = rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| {
                is_soname(n)
                    && GPU_DRIVER_LIB_PREFIXES
                        .iter()
                        .any(|p| n.starts_with(p))
            })
            .collect();
        names.sort();
        for n in names {
            if libs.len() >= MAX_GPU_LIBS {
                break;
            }
            let guest = format!("/usr/local/lib/{n}");
            if libs.iter().any(|(_, g)| *g == guest) {
                continue;
            }
            libs.push((format!("{dir}/{n}"), guest));
        }
    }
    (devices, libs)
}

pub fn create(
    root: &Path,
    image: &str,
    worktree_id: &str,
    branch_id: &str,
) -> Result<String, String> {
    let payload = json!({ "image": image, "network": true });

    let mount = crate::hostgit::mount_for(root, worktree_id, branch_id);
    let mut volumes: Vec<Value> = Vec::new();
    if let Some((path, _)) = &mount {
        volumes.push(json!({
            "type": "bind",
            "path": path,
            // One fixed guest path, so an agent's instructions do not have
            // is angela's because the other half of the bargain is there:
            "mount": angela::harness::SANDBOX_MOUNT,
            "mode": "rw",
        }));
    }

    // GPU : requested, never load-bearing. If the devices make
    let (devices, gpu_libs) = gpu_setup();
    for (host, guest) in &gpu_libs {
        volumes.push(json!({
            "type": "bind",
            "path": host,
            "mount": guest,
            "mode": "ro",
        }));
    }
    let had_devices = !devices.is_empty();
    let create_with = |devices: Value, volumes: Value| -> Result<String, String> {
        let mut p = payload.clone();
        if !devices.is_null() {
            p["devices"] = devices;
        }
        // and must not inherit the first attempt's GPU bind.
        p["volumes"] = if volumes.is_null() { json!([]) } else { volumes };
        let created = request("POST", "/api/orchestration/sandboxes", Some(&p))?;
        created
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .ok_or_else(|| "sbox returned no id".to_string())
    };
    let dev_json = if had_devices { json!(devices) } else { Value::Null };
    let vol_json = if volumes.is_empty() { Value::Null } else { json!(volumes) };
    let id = match create_with(dev_json, vol_json) {
        Ok(id) => id,
        Err(first) if had_devices => {
            let plain: Vec<Value> = volumes
                .iter()
                .filter(|v| {
                    !v.get("mount")
                        .and_then(Value::as_str)
                        .is_some_and(|m| m.starts_with("/usr/local/lib/"))
                })
                .cloned()
                .collect();
            let plain = if plain.is_empty() { Value::Null } else { json!(plain) };
            create_with(Value::Null, plain)
                .map_err(|e| format!("{first} (and without GPU devices: {e})"))?
        }
        Err(e) => return Err(e),
    };

    // Tie the checkout to the box, so destroying the box removes it. Only a
    if let Some((_, wt)) = &mount {
        if !wt.is_empty() {
            crate::hostgit::claim_worktree(root, wt, &id);
        }
    }
    Ok(id)
}

/// when sbox-b no longer tracks it (deleted, or it never came up).
pub fn state(id: &str) -> Option<String> {
    let v = request("GET", &format!("/api/orchestration/sandboxes/{id}"), None).ok()?;
    v.get("state").and_then(Value::as_str).map(str::to_string)
}

pub fn rows() -> Result<Vec<SandboxRow>, String> {
    let v = request("GET", "/api/orchestration/sandboxes", None)?;
    let list = v.get("sandboxes").and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(list
        .iter()
        .map(|s| SandboxRow {
            id: s.get("id").and_then(Value::as_str).unwrap_or_default().to_string(),
            image: s.get("image").and_then(Value::as_str).unwrap_or("unknown").to_string(),
            state: s.get("state").and_then(Value::as_str).unwrap_or("unknown").to_string(),
            created_at: s
                .get("createdAt")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
        .collect())
}

pub fn destroy(root: &Path, id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("which sandbox?".into());
    }
    request("DELETE", &format!("/api/orchestration/sandboxes/{id}"), None)?;
    crate::hostgit::forget_for_sandbox(root, id);
    Ok(())
}

pub struct SboxBackend {
    pub root: std::path::PathBuf,
}

/// because the `exec` argv dialect is identical across the three.
fn runtime_bin() -> String {
    if let Ok(bin) = std::env::var("SANDBOX_RUNTIME") {
        if !bin.trim().is_empty() {
            return bin;
        }
    }
    for bin in ["container", "podman", "docker"] {
        let found = std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .filter(|d| !d.is_empty())
            .any(|d| std::fs::metadata(format!("{d}/{bin}")).is_ok());
        if found {
            return bin.to_string();
        }
    }
    "podman".to_string()
}

impl angela::sandbox::Backend for SboxBackend {
    fn create(&self, image: &str, worktree_id: &str, branch_id: &str) -> Result<String, String> {
        create(&self.root, image, worktree_id, branch_id)
    }
    fn state(&self, id: &str) -> Option<String> {
        state(id)
    }
    fn destroy(&self, id: &str) -> Result<(), String> {
        destroy(&self.root, id)
    }
    fn list(&self) -> Result<Vec<SandboxRow>, String> {
        rows()
    }
    fn exec_argv(&self, container: &str, guest: &[String], env: &[(String, String)]) -> Vec<String> {
        let mut argv = vec![runtime_bin(), "exec".into(), "-i".into()];
        for (k, v) in env {
            argv.push("-e".into());
            argv.push(format!("{k}={v}"));
        }
        argv.push(container.to_string());
        argv.extend(guest.iter().cloned());
        argv
    }
}