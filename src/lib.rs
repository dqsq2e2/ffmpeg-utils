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
    let method_str = match CStr::from_ptr(method as *const i8).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let params_str = match CStr::from_ptr(params as *const i8).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let params_json: Value = match serde_json::from_str(params_str) {
        Ok(v) => v,
        Err(_) => return -1,
    };

    let result = match method_str {
        "get_ffmpeg_path" => get_ffmpeg_path(),
        "get_ffprobe_path" => get_ffprobe_path(),
        "check_version" => check_version(),
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
        let _ = CString::from_raw(ptr as *mut i8);
    }
}

// --- Implementation ---

fn get_bin_path(binary_name: &str) -> Option<PathBuf> {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(root) = current_exe.parent() {
            // Plugins structure:
            // plugins/
            //   ffmpeg-utils/
            //     ffmpeg_utils.dll
            //     bin/
            //       ffmpeg.exe
            //       ffprobe.exe
            
            // Adjust based on typical layout
            let plugin_dir = root.join("plugins").join("ffmpeg-utils");
            
            // On some systems it might be directly in plugin dir or in bin
            // Check ./bin/ first
            let mut path = plugin_dir.join("bin").join(binary_name);
            
            // Handle extension
            if let Some(ext) = std::env::consts::EXE_EXTENSION.is_empty().then(|| "").or(Some(std::env::consts::EXE_EXTENSION)) {
                 if !ext.is_empty() {
                     path.set_extension(ext);
                 }
            }
            
            if path.exists() {
                return Some(path);
            }
            
            // Check plugin root
            let mut path = plugin_dir.join(binary_name);
             if let Some(ext) = std::env::consts::EXE_EXTENSION.is_empty().then(|| "").or(Some(std::env::consts::EXE_EXTENSION)) {
                 if !ext.is_empty() {
                     path.set_extension(ext);
                 }
            }
            if path.exists() {
                return Some(path);
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
