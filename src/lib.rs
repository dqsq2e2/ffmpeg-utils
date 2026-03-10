use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::path::PathBuf;
use serde_json::{json, Value};

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

    let _params_str = match CStr::from_ptr(params as *const std::os::raw::c_char).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let result = match method_str {
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

fn get_bin_path(binary_name: &str) -> Option<PathBuf> {
    let mut search_paths = Vec::new();

    // 1. Check relative to CWD (Development & typical run)
    if let Ok(cwd) = std::env::current_dir() {
        search_paths.push(cwd.join("plugins").join("ffmpeg-utils"));
        search_paths.push(cwd.join("plugins").join("ffmpeg-utils").join("bin"));
        search_paths.push(cwd.join("backend").join("plugins").join("ffmpeg-utils"));
        search_paths.push(cwd.join("backend").join("plugins").join("ffmpeg-utils").join("bin"));
        // Also check root/bin (common in some setups)
        search_paths.push(cwd.join("bin")); 
    }

    // 2. Check relative to Executable (Production / Release)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(root) = current_exe.parent() {
            // Standard deployment: plugins/ffmpeg-utils/
            search_paths.push(root.join("plugins").join("ffmpeg-utils"));
            search_paths.push(root.join("plugins").join("ffmpeg-utils").join("bin"));
            // Flat deployment
            search_paths.push(root.to_path_buf());
            search_paths.push(root.join("bin"));
        }
    }

    let exe_ext = if cfg!(windows) { "exe" } else { "" };

    for dir in search_paths {
        let mut path = dir.join(binary_name);
        if !exe_ext.is_empty() {
            path.set_extension(exe_ext);
        }
        
        if path.exists() {
            return Some(path);
        }
    }

    // 3. Fallback: Check if it's in PATH
    // We can't easily check PATH without running 'which'/'where', 
    // but we can return the bare command if we want to rely on system PATH.
    // However, the caller expects a valid path.
    // Let's assume if we can't find it, we return None and let the error propagate.
    
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
