use lazy_static::lazy_static;
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

lazy_static! {
    static ref PLUGIN_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);
}

// --- FFI Interface ---

#[no_mangle]
pub unsafe extern "C" fn plugin_invoke(
    method: *const u8,
    params: *const u8,
    result_ptr: *mut *mut u8,
) -> c_int {
    let method_str = match CStr::from_ptr(method as *const std::os::raw::c_char).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let params_str = match CStr::from_ptr(params as *const std::os::raw::c_char).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let result = match method_str {
        "initialize" => initialize(params_str),
        "execute" => execute(params_str),
        "get_ffmpeg_path" => get_ffmpeg_path(),
        "get_ffprobe_path" => get_ffprobe_path(),
        "check_version" => check_version(),
        "garbage_collect" => Ok(json!({})),
        _ => Err(format!("Unknown method: {}", method_str)),
    };

    match result {
        Ok(val) => {
            let json = serde_json::to_string(&val).unwrap_or_default();
            let c_string = match CString::new(json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            *result_ptr = c_string.into_raw() as *mut u8;
            0 // Success
        }
        Err(e) => {
            let error_json = json!({ "error": e }).to_string();
            let c_string = match CString::new(error_json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            *result_ptr = c_string.into_raw() as *mut u8;
            -1 // Failure
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn plugin_free(ptr: *mut u8) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr as *mut std::os::raw::c_char);
    }
}

// --- Implementation ---

fn initialize(params_str: &str) -> Result<Value, String> {
    let params: Value = serde_json::from_str(params_str).map_err(|e| e.to_string())?;

    if let Some(plugin_path_str) = params.get("plugin_path").and_then(|v| v.as_str()) {
        let path = PathBuf::from(plugin_path_str);
        if let Ok(mut lock) = PLUGIN_DIR.write() {
            *lock = Some(path);
        }
    }

    Ok(json!({ "status": "initialized" }))
}

fn candidate(root: &Path, binary_name: &str) -> Option<PathBuf> {
    let mut direct = root.join(binary_name);
    if cfg!(windows) {
        direct.set_extension("exe");
    }
    if direct.exists() {
        return Some(direct);
    }

    let mut bundled = root.join("bin").join(binary_name);
    if cfg!(windows) {
        bundled.set_extension("exe");
    }
    bundled.exists().then_some(bundled)
}

fn get_bin_path(binary_name: &str) -> Option<PathBuf> {
    let mut search_paths = Vec::new();

    if let Ok(lock) = PLUGIN_DIR.read() {
        if let Some(path) = lock.as_ref() {
            search_paths.push(path.clone());
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(root) = current_exe.parent() {
            search_paths.push(root.to_path_buf());
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        search_paths.push(cwd);
    }

    for root in &search_paths {
        if let Some(path) = candidate(root, binary_name) {
            return Some(path);
        }
    }

    for root in &search_paths {
        let plugin_roots = [
            root.join("plugins"),
            root.join("backend").join("plugins"),
            root.join("ting-reader").join("backend").join("plugins"),
            root.join("..").join("..").join("plugins"),
        ];

        for plugins_dir in plugin_roots {
            if plugins_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_dir()
                            && path.file_name().is_some_and(|name| name == "ffmpeg-utils")
                        {
                            if let Some(path) = candidate(&path, binary_name) {
                                return Some(path);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn get_ffmpeg_path() -> Result<Value, String> {
    if let Some(path) = get_bin_path("ffmpeg") {
        Ok(json!({ "path": path.to_string_lossy().to_string() }))
    } else {
        Err("FFmpeg binary not found in plugin directory".to_string())
    }
}

fn get_ffprobe_path() -> Result<Value, String> {
    if let Some(path) = get_bin_path("ffprobe") {
        Ok(json!({ "path": path.to_string_lossy().to_string() }))
    } else {
        Err("FFprobe binary not found in plugin directory".to_string())
    }
}

fn check_version() -> Result<Value, String> {
    // Just return success for now
    Ok(json!({ "status": "ok", "version": "unknown" }))
}

fn execute(params_str: &str) -> Result<Value, String> {
    let params: Value = serde_json::from_str(params_str).map_err(|e| e.to_string())?;
    let tool_name = params
        .get("name")
        .or_else(|| params.get("tool"))
        .or_else(|| params.get("tool_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("ffmpeg.provider");

    match tool_name {
        "ffmpeg.provider" => Ok(json!({
            "tool": tool_name,
            "ffmpeg": path_or_error("ffmpeg"),
            "ffprobe": path_or_error("ffprobe"),
            "version": check_version().unwrap_or_else(|error| json!({ "error": error })),
        })),
        "ffmpeg.get_path" => get_ffmpeg_path(),
        "ffprobe.get_path" => get_ffprobe_path(),
        "ffmpeg.check_version" => check_version(),
        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}

fn path_or_error(binary_name: &str) -> Value {
    match get_bin_path(binary_name) {
        Some(path) => json!({
            "available": true,
            "path": path.to_string_lossy().to_string(),
        }),
        None => json!({
            "available": false,
            "error": format!("{} binary not found in plugin directory", binary_name),
        }),
    }
}
