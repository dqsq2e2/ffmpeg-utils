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

#[cfg(windows)]
fn get_dll_dir() -> Option<PathBuf> {
    None
}

#[cfg(not(windows))]
fn get_dll_dir() -> Option<PathBuf> {
    None
}

fn get_bin_path(binary_name: &str) -> Option<PathBuf> {
    let mut search_paths = Vec::new();

    // 1. Check relative to Executable (Production usually)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(root) = current_exe.parent() {
            search_paths.push(root.to_path_buf());
        }
    }

    // 2. Check relative to CWD (Development usually)
    if let Ok(cwd) = std::env::current_dir() {
        search_paths.push(cwd);
    }

    let exe_ext = if cfg!(windows) { "exe" } else { "" };

    for root in search_paths {
        // Try to find "plugins" directory
        let possible_plugin_dirs = vec![
            root.join("plugins"),
            root.join("backend").join("plugins"),
            root.join("ting-reader").join("backend").join("plugins"),
            // Case: running from target/debug/deps, so plugins is up 3 levels then plugins
            root.join("..").join("..").join("plugins"), 
        ];

        for plugins_dir in possible_plugin_dirs {
            if plugins_dir.exists() {
                // Look for any folder starting with "FFmpeg Provider" or "ffmpeg-utils"
                if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_dir() {
                            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            if dir_name.starts_with("FFmpeg Provider") || dir_name.starts_with("ffmpeg-utils") {
                                // Found candidate directory, check for binary
                                let mut bin_path = path.join(binary_name);
                                if !exe_ext.is_empty() { bin_path.set_extension(exe_ext); }
                                if bin_path.exists() { return Some(bin_path); }

                                let mut bin_sub_path = path.join("bin").join(binary_name);
                                if !exe_ext.is_empty() { bin_sub_path.set_extension(exe_ext); }
                                if bin_sub_path.exists() { return Some(bin_sub_path); }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Check ./(binary).exe (CWD root fallback)
    let mut local_path = PathBuf::from(binary_name);
    if !exe_ext.is_empty() { local_path.set_extension(exe_ext); }
    if local_path.exists() {
        return Some(local_path);
    }

    // Default to system PATH
    Some(PathBuf::from(binary_name))
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
