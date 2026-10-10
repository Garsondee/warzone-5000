//! A tiny HTTP/1.1 server on `std::net` only (one thread per connection, `Connection: close`), loopback only. It serves the prebuilt page
//! from the `--web` folder and the JSON API of `docs/swarm/requests/arch-drive-protocol.md`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::assist::Raw;
use super::live::{Cmd, Shared};
use super::session::Car;

const MAX_BODY: usize = 64 * 1024; // const-ok: a request body is a few dozen bytes; this bounds a bad client
const MAX_HEADERS: usize = 64; // const-ok: header lines accepted before giving up
const READ_TIMEOUT: Duration = Duration::from_secs(10); // const-ok: a client that sends nothing for this long is dropped
const KEEPALIVE: Duration = Duration::from_secs(1); // const-ok: an idle stream gets a comment line this often (detects closed browsers)
const FINISH_WAIT: Duration = Duration::from_secs(10); // const-ok: how long /api/finish waits for the simulation thread

/// Everything the connection threads need.
pub(crate) struct App {
    pub garage: Vec<Arc<Car>>,
    pub terrain_json: String,
    pub shared: Arc<Shared>,
    /// The folder of the prebuilt page, if one was found.
    pub web: Option<PathBuf>,
}

struct Req {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_request(stream: &TcpStream) -> Result<Req, String> {
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    r.by_ref().take(MAX_BODY as u64).read_line(&mut line).map_err(|e| e.to_string())?;
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().ok_or("empty request")?, parts.next().ok_or("no path")?);
    let mut length = 0usize;
    for _ in 0..MAX_HEADERS {
        let mut h = String::new();
        r.by_ref().take(MAX_BODY as u64).read_line(&mut h).map_err(|e| e.to_string())?;
        let h = h.trim_end();
        if h.is_empty() {
            let mut body = vec![0u8; length.min(MAX_BODY)];
            r.read_exact(&mut body).map_err(|e| e.to_string())?;
            return Ok(Req {
                method: method.to_string(),
                path: target.split('?').next().unwrap_or("/").to_string(),
                body,
            });
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                length = v.trim().parse().map_err(|_| "bad Content-Length")?;
            }
        }
    }
    Err("too many headers".into())
}

fn respond(w: &mut impl Write, status: u16, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    write!(
        w,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    w.write_all(body)
}

fn json_reply(w: &mut impl Write, status: u16, v: &Value) -> std::io::Result<()> {
    respond(w, status, "application/json", v.to_string().as_bytes())
}

fn error(w: &mut impl Write, status: u16, msg: &str) -> std::io::Result<()> {
    json_reply(w, status, &json!({ "error": msg }))
}

/// The body of `POST /api/input`: absent fields count as zero / false (the body replaces the whole input).
fn parse_input(body: &[u8]) -> Result<(Raw, bool), String> {
    let v: Value = serde_json::from_slice(body).map_err(|e| format!("bad JSON: {e}"))?;
    let obj = v.as_object().ok_or("the body must be a JSON object")?;
    let num = |k: &str| match obj.get(k) {
        None => Ok(0.0),
        Some(x) => x.as_f64().ok_or(format!("{k} must be a number")),
    };
    let flag = |k: &str| obj.get(k).and_then(Value::as_bool).unwrap_or(false);
    Ok((
        Raw { throttle: num("throttle")?, brake: num("brake")?, steer: num("steer")?, reverse: flag("reverse") },
        flag("reset"),
    ))
}

fn content_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "glb" => "model/gltf-binary",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn serve_static(w: &mut impl Write, app: &App, url_path: &str) -> std::io::Result<()> {
    let Some(root) = &app.web else {
        return respond(
            w,
            404,
            "text/plain",
            b"w5k drive: no web page found (use --web DIR, or build tools/viewer/dist)",
        );
    };
    let rel = url_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if rel.split('/').any(|c| c == ".." || c.contains(['\\', ':', '%', '\0'])) {
        return error(w, 400, "bad path");
    }
    let file = root.join(rel);
    match std::fs::read(&file) {
        Ok(bytes) => respond(w, 200, content_type(&file), &bytes),
        Err(_) => error(w, 404, "not found"),
    }
}

/// Decrements the client count when the stream ends, however it ends.
struct ClientGuard(Arc<Shared>);

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.0.clients.fetch_sub(1, Ordering::Relaxed);
    }
}

fn stream_events(mut w: TcpStream, app: &App) {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
    app.shared.clients.fetch_add(1, Ordering::Relaxed);
    let _guard = ClientGuard(app.shared.clone());
    let mut next = app.shared.subscribe();
    if w.write_all(head.as_bytes()).is_err() {
        return;
    }
    while !app.shared.stop.load(Ordering::Relaxed) {
        let (frames, n) = app.shared.frames_since(next, KEEPALIVE);
        next = n;
        let ok = if frames.is_empty() {
            w.write_all(b": keep-alive\n\n")
        } else {
            frames.iter().try_for_each(|f| write!(w, "data: {f}\n\n"))
        };
        if ok.and_then(|()| w.flush()).is_err() {
            return;
        }
    }
}

fn route(mut stream: TcpStream, req: Req, app: &App) -> std::io::Result<()> {
    let w = &mut stream;
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/stream") => {
            stream_events(stream, app);
            Ok(())
        }
        ("GET", "/api/vehicles") => {
            let mm = |v: f64| (v * 1e3).round() / 1e3; // const-ok: millimetre / gram rounding for display
            let list: Vec<Value> = app
                .garage
                .iter()
                .map(|c| json!({"id": c.id, "name": c.name, "mass_kg": mm(c.mass_kg), "wheelbase_m": mm(c.wheelbase_m), "skin": c.id}))
                .collect();
            json_reply(w, 200, &Value::Array(list))
        }
        ("GET", "/api/world") => respond(w, 200, "application/json", app.terrain_json.as_bytes()),
        ("GET", p) if p.starts_with("/api/rig/") => match app.garage.iter().find(|c| c.id == p["/api/rig/".len()..]) {
            Some(c) => respond(w, 200, "application/json", c.render_json.as_bytes()),
            None => error(w, 404, "unknown vehicle"),
        },
        ("POST", "/api/select") => {
            let id =
                serde_json::from_slice::<Value>(&req.body).ok().and_then(|v| v["vehicle"].as_str().map(String::from));
            match id.filter(|id| app.garage.iter().any(|c| &c.id == id)) {
                Some(id) => {
                    app.shared.push_cmd(Cmd::Select(id));
                    json_reply(w, 200, &json!({"ok": true}))
                }
                None => error(w, 404, "unknown vehicle"),
            }
        }
        ("POST", "/api/input") => match parse_input(&req.body) {
            Ok((raw, reset)) => {
                app.shared.set_input(raw);
                if reset {
                    app.shared.push_cmd(Cmd::Reset);
                }
                json_reply(w, 200, &json!({"ok": true}))
            }
            Err(e) => error(w, 400, &e),
        },
        ("POST" | "GET", "/api/finish") => {
            let (tx, rx) = channel();
            app.shared.push_cmd(Cmd::Finish(tx));
            match rx.recv_timeout(FINISH_WAIT) {
                Ok(Ok(file)) => json_reply(w, 200, &json!({"ok": true, "file": file})),
                Ok(Err(e)) => error(w, 400, &e),
                Err(_) => error(w, 400, "the simulation did not answer"),
            }
        }
        ("GET", p) if !p.starts_with("/api/") => serve_static(w, app, p),
        ("GET" | "POST", _) => error(w, 404, "not found"),
        _ => error(w, 405, "method not allowed"),
    }
}

fn handle(stream: TcpStream, app: &App) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let _ = stream.set_nodelay(true);
    match read_request(&stream) {
        Ok(req) => {
            let _ = route(stream, req, app);
        }
        Err(e) => {
            let mut s = stream;
            let _ = error(&mut s, 400, &e);
        }
    }
}

/// Accept connections until the process ends (one thread each).
pub(crate) fn serve(listener: TcpListener, app: Arc<App>) {
    for conn in listener.incoming().flatten() {
        let app = app.clone();
        std::thread::spawn(move || handle(conn, &app));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_input_body_replaces_the_whole_input_and_absent_fields_count_as_released() {
        let (raw, reset) = parse_input(br#"{"throttle": 0.5, "steer": -1}"#).expect("parses");
        assert_eq!((raw.throttle, raw.brake, raw.steer, raw.reverse, reset), (0.5, 0.0, -1.0, false, false));
        let (raw, reset) = parse_input(br#"{"reset": true, "reverse": true}"#).expect("parses");
        assert!(reset && raw.reverse && raw.throttle.abs() < 1e-12);
    }

    #[test]
    fn a_malformed_input_body_is_refused_with_a_reason() {
        for bad in [&b"nope"[..], b"[1]", br#"{"throttle": "fast"}"#] {
            assert!(parse_input(bad).is_err(), "{}", String::from_utf8_lossy(bad));
        }
    }

    #[test]
    fn static_files_get_their_content_type_and_paths_cannot_leave_the_web_folder() {
        let dir = std::env::temp_dir().join(format!("w5k_drive_http_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).expect("dir");
        std::fs::write(dir.join("sub/app.js"), "1").expect("file");
        let app =
            App { garage: Vec::new(), terrain_json: String::new(), shared: Arc::default(), web: Some(dir.clone()) };
        let reply = |path: &str| {
            let mut out = Vec::new();
            serve_static(&mut out, &app, path).expect("write");
            String::from_utf8_lossy(&out).into_owned()
        };
        assert!(
            reply("/sub/app.js").starts_with("HTTP/1.1 200 OK") && reply("/sub/app.js").contains("text/javascript")
        );
        for bad in ["/../secret", "/sub/..%2f..", "/C:/Windows", "/a\\b"] {
            assert!(reply(bad).starts_with("HTTP/1.1 400"), "{bad}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
