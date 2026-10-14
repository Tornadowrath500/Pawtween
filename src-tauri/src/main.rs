#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager, State};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn ffmpeg_path(app: &tauri::AppHandle) -> PathBuf {
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if let Ok(d) = app.path().resource_dir() {
        let p = d.join("binaries").join(name);
        if p.exists() { return p; }
    }
    PathBuf::from(name) // falls back to ffmpeg on PATH (handy in dev)
}

/// Builds the ffmpeg command. `low` runs it at a lower priority so the rest of the computer stays smooth.
fn new_cmd(app: &tauri::AppHandle, low: bool) -> Command {
    #[cfg(unix)]
    let mut c = if low {
        let mut n = Command::new("nice");
        n.arg("-n").arg("10").arg(ffmpeg_path(app));
        n
    } else { Command::new(ffmpeg_path(app)) };
    #[cfg(windows)]
    let mut c = Command::new(ffmpeg_path(app));
    #[cfg(windows)]
    c.creation_flags(0x08000000 | if low { 0x00004000 } else { 0 }); // no console window, BELOW_NORMAL priority
    c
}

/// One running FFmpeg process that frames are streamed into. Several can run at once (editor export + render queue).
struct Enc {
    child: Child,
    stdin: Option<ChildStdin>,
    err: Arc<Mutex<String>>,
    out: String,
}
#[derive(Default)]
struct EncState(Mutex<HashMap<String, Enc>>);
#[derive(Default)]
struct OpenFile(Mutex<Option<String>>);

/// Finds a working hardware H.264 encoder (NVIDIA, AMD or Intel). Empty string means use the CPU.
#[tauri::command(async)]
fn probe_hw(app: tauri::AppHandle) -> String {
    for enc in ["h264_nvenc", "h264_amf", "h264_qsv"] {
        let mut c = new_cmd(&app, false);
        c.args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.2", "-c:v", enc, "-f", "null", "-"]);
        c.stdout(Stdio::null()).stderr(Stdio::null());
        if let Ok(s) = c.status() {
            if s.success() { return enc.to_string(); }
        }
    }
    String::new()
}

#[tauri::command(async)]
fn enc_start(app: tauri::AppHandle, state: State<'_, EncState>, id: String, fps: f64, w: u32, h: u32, out: String,
             audio: Option<String>, fmt: String, crf: u32, hw: Option<String>, threads: Option<u32>, low: Option<bool>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut old) = guard.remove(&id) { let _ = old.child.kill(); }
    let audio = if matches!(fmt.as_str(), "gif" | "apng" | "seq") { None } else { audio };
    let size = format!("{}x{}", w, h);
    let fps_s = fps.to_string();
    let mut c = new_cmd(&app, low.unwrap_or(false));
    c.args(["-y", "-hide_banner", "-loglevel", "error", "-nostats", "-f", "rawvideo", "-pix_fmt", "rgba", "-s", size.as_str(), "-framerate", fps_s.as_str(), "-i", "-"]);
    if let Some(a) = &audio {
        c.args(["-f", "f32le", "-ar", "48000", "-ac", "2", "-i", a.as_str()]);
    }
    let th = threads.unwrap_or(0);
    if th > 0 { c.args(["-threads", th.to_string().as_str()]); }
    let hwn = hw.unwrap_or_default();
    let q = crf.to_string();
    let q4 = (crf + 4).to_string();
    match fmt.as_str() {
        "webm" => {
            c.args(["-c:v", "libvpx-vp9", "-pix_fmt", "yuva420p", "-b:v", "0", "-crf", "26", "-auto-alt-ref", "0", "-row-mt", "1", "-deadline", "good", "-cpu-used", "3"]);
            if audio.is_some() { c.args(["-c:a", "libopus", "-b:a", "160k"]); }
        }
        "h265" => {
            c.args(["-c:v", "libx265", "-preset", "veryfast", "-crf", q4.as_str(), "-pix_fmt", "yuv420p", "-tag:v", "hvc1", "-movflags", "+faststart"]);
            if audio.is_some() { c.args(["-c:a", "aac", "-b:a", "192k"]); }
        }
        "mov" => {
            c.args(["-c:v", "prores_ks", "-profile:v", "4444", "-pix_fmt", "yuva444p10le"]);
            if audio.is_some() { c.args(["-c:a", "pcm_s16le"]); }
        }
        "apng" => { c.args(["-f", "apng", "-plays", "0"]); }
        "seq" => {}
        "gif" => { c.args(["-vf", "fps=24,split[a][b];[a]palettegen[p];[b][p]paletteuse"]); }
        _ => {
            match hwn.as_str() {
                "h264_nvenc" => { c.args(["-c:v", "h264_nvenc", "-preset", "p4", "-rc", "vbr", "-cq", q.as_str(), "-b:v", "0", "-pix_fmt", "yuv420p", "-movflags", "+faststart"]); }
                "h264_amf" => { c.args(["-c:v", "h264_amf", "-quality", "speed", "-rc", "cqp", "-qp_i", q.as_str(), "-qp_p", q.as_str(), "-pix_fmt", "yuv420p", "-movflags", "+faststart"]); }
                "h264_qsv" => { c.args(["-c:v", "h264_qsv", "-global_quality", q.as_str(), "-pix_fmt", "nv12", "-movflags", "+faststart"]); }
                _ => { c.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", q.as_str(), "-pix_fmt", "yuv420p", "-movflags", "+faststart"]); }
            }
            if audio.is_some() { c.args(["-c:a", "aac", "-b:a", "192k"]); }
        }
    }
    let out_arg = if fmt == "seq" {
        format!("{}_%05d.png", std::path::Path::new(&out).with_extension("").to_string_lossy())
    } else { out.clone() };
    c.arg(&out_arg);
    c.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = c.spawn().map_err(|e| format!("Could not start ffmpeg: {e}"))?;
    let stdin = child.stdin.take();
    let err = Arc::new(Mutex::new(String::new()));
    if let Some(mut se) = child.stderr.take() {
        let e2 = err.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match se.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut s) = e2.lock() {
                            if s.len() < 8000 { s.push_str(&String::from_utf8_lossy(&buf[..n])); }
                        }
                    }
                }
            }
        });
    }
    guard.insert(id, Enc { child, stdin, err, out });
    Ok(())
}

/// Receives one raw RGBA frame (job id in the `x-job` header) and writes it into that job's FFmpeg.
#[tauri::command(async)]
fn enc_frame(state: State<'_, EncState>, request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let data = match request.body() {
        tauri::ipc::InvokeBody::Raw(d) => d,
        _ => return Err("expected raw frame bytes".into()),
    };
    let id = request.headers().get("x-job").and_then(|v| v.to_str().ok()).unwrap_or("main").to_string();
    let mut g = state.0.lock().map_err(|e| e.to_string())?;
    let enc = g.get_mut(&id).ok_or("the encoder is not running")?;
    let err = enc.err.clone();
    let si = enc.stdin.as_mut().ok_or("the encoder input is closed")?;
    si.write_all(data).map_err(|e| {
        let m = err.lock().map(|s| s.clone()).unwrap_or_default();
        format!("ffmpeg stopped: {e}. {m}")
    })
}

#[tauri::command(async)]
fn enc_finish(state: State<'_, EncState>, id: String) -> Result<String, String> {
    let mut enc = {
        let mut g = state.0.lock().map_err(|e| e.to_string())?;
        g.remove(&id).ok_or("the encoder is not running")?
    };
    drop(enc.stdin.take());
    let st = enc.child.wait().map_err(|e| e.to_string())?;
    std::thread::sleep(std::time::Duration::from_millis(40));
    if st.success() { Ok(enc.out) } else { Err(enc.err.lock().map(|s| s.clone()).unwrap_or_default()) }
}

#[tauri::command(async)]
fn enc_cancel(state: State<'_, EncState>, id: String) -> Result<(), String> {
    let mut g = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut e) = g.remove(&id) {
        drop(e.stdin.take());
        let _ = e.child.kill();
    }
    Ok(())
}

/// Shows a file in Explorer / Finder / the file manager.
#[tauri::command]
fn reveal(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let p = path.replace('/', "\\");
        let _ = Command::new("explorer").raw_arg(format!("/select,\"{}\"", p)).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").args(["-R", path.as_str()]).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let p = std::path::Path::new(&path);
        let dir = p.parent().unwrap_or(p);
        let _ = Command::new("xdg-open").arg(dir).spawn();
    }
    Ok(())
}

/// The project file the app was launched with (double-click in Explorer or Finder), if any.
#[tauri::command]
fn get_open_file(state: State<'_, OpenFile>) -> Option<String> {
    state.0.lock().ok().and_then(|mut g| g.take())
}

/// Reads a .pawtween / .pawtweenpack file as raw bytes (only those two types).
#[tauri::command(async)]
fn read_file(path: String) -> Result<tauri::ipc::Response, String> {
    let l = path.to_lowercase();
    if !(l.ends_with(".pawtween") || l.ends_with(".pawtweenpack")) { return Err("not a Pawtween file".into()); }
    let b = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(b))
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(EncState::default())
        .manage(OpenFile::default())
        .setup(|app| {
            let a = std::env::args().skip(1).find(|a| {
                let l = a.to_lowercase();
                l.ends_with(".pawtween") || l.ends_with(".pawtweenpack")
            });
            if let Some(a) = a {
                if let Ok(mut g) = app.state::<OpenFile>().0.lock() { *g = Some(a); }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![probe_hw, enc_start, enc_frame, enc_finish, enc_cancel, reveal, get_open_file, read_file])
        .build(tauri::generate_context!())
        .expect("error while building Pawtween");
    app.run(|_handle, _event| {
        // macOS delivers double-clicked files as an event instead of a command-line argument
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = &_event {
            if let Some(p) = urls.iter().filter_map(|u| u.to_file_path().ok()).next() {
                let p = p.to_string_lossy().to_string();
                if let Ok(mut g) = _handle.state::<OpenFile>().0.lock() { *g = Some(p.clone()); }
                let _ = _handle.emit("open-file", p);
            }
        }
    });
}
