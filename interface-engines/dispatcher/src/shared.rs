use anyhow::{anyhow, Result};
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};

pub(crate) const PRODUCT: &str = "1BitShit CPU";

pub(crate) type InstantiateFn =
    unsafe extern "C" fn(*const c_char, *const c_void) -> *mut c_void;
pub(crate) type FreeFn = unsafe extern "C" fn(*mut c_void);
pub(crate) type StreamCallback = extern "C" fn(*const c_char, *mut c_void) -> bool;
pub(crate) type GenerateStreamFn = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    usize,
    StreamCallback,
    *mut c_void,
) -> i32;
pub(crate) type InitFn = unsafe extern "C" fn() -> *const c_char;
pub(crate) type GenerateEmbeddingFn = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut f32,
    usize,
    *mut usize,
) -> i32;

fn sanitized_model_id(id: &str) -> String {
    id.chars()
        .map(|character| match character {
            ':' | '/' | '\\' | ' ' => '-',
            other => other,
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub(crate) fn configured_model_id(kind: &str) -> Option<String> {
    let environment = cluaiz_shared::environment::EnvironmentManager::current();
    let permission_path = environment.config_dir().join("Permission.json");
    let content = std::fs::read_to_string(permission_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get(kind)?
        .get("text")?
        .as_str()
        .map(str::to_string)
}

pub(crate) fn find_model_file(
    model_id: &str,
    accepted_extensions: &[&str],
) -> Option<PathBuf> {
    let environment = cluaiz_shared::environment::EnvironmentManager::current();
    let models_root = environment.models_dir();
    let directory_name = sanitized_model_id(model_id);

    for category in ["chat", "embedding", "vision", "audio", "code", "multimodal"] {
        let directory = models_root.join(category).join(&directory_name);
        if let Some(path) = find_weight_in_directory(&directory, accepted_extensions) {
            return Some(path);
        }
    }

    find_weight_recursive(&models_root, &directory_name, accepted_extensions)
}

fn find_weight_in_directory(directory: &Path, accepted_extensions: &[&str]) -> Option<PathBuf> {
    std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase)
                    .map(|extension| accepted_extensions.contains(&extension.as_str()))
                    .unwrap_or(false)
        })
}

fn find_weight_recursive(
    root: &Path,
    directory_name: &str,
    accepted_extensions: &[&str],
) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case(directory_name))
            .unwrap_or(false)
        {
            if let Some(weight) = find_weight_in_directory(&path, accepted_extensions) {
                return Some(weight);
            }
        }

        if let Some(weight) = find_weight_recursive(&path, directory_name, accepted_extensions) {
            return Some(weight);
        }
    }
    None
}

fn library_extension() -> &'static str {
    if cfg!(windows) {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

fn library_names(kind: &str) -> Vec<String> {
    let extension = library_extension();
    if cfg!(windows) {
        vec![
            format!("bitshit-{kind}.{extension}"),
            format!("bitshit_{kind}.{extension}"),
            format!("cluaiz-{kind}.{extension}"),
            format!("cluaiz_{kind}.{extension}"),
        ]
    } else {
        vec![
            format!("bitshit-{kind}.{extension}"),
            format!("libbitshit_{kind}.{extension}"),
            format!("libbitshit-{kind}.{extension}"),
            format!("cluaiz-{kind}.{extension}"),
            format!("libcluaiz_{kind}.{extension}"),
            format!("libcluaiz-{kind}.{extension}"),
        ]
    }
}

pub(crate) fn resolve_library(kind: &str) -> Result<PathBuf> {
    let names = library_names(kind);
    let environment = cluaiz_shared::environment::EnvironmentManager::current();
    let installed_dir = environment.engine_dir();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for profile in ["release", "debug"] {
        let target = current_dir.join("target").join(profile);
        for name in &names {
            for candidate in [target.join(name), target.join("deps").join(name)] {
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }

    for name in &names {
        for candidate in [installed_dir.join(name), installed_dir.join("drivers").join(name)] {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(anyhow!(
        "{} {} library was not found under {} or target/{{release,debug}}",
        PRODUCT,
        kind,
        installed_dir.display()
    ))
}

pub(crate) unsafe fn load_symbol<'library, T>(
    library: &'library libloading::Library,
    primary: &[u8],
    legacy: &[u8],
) -> Result<libloading::Symbol<'library, T>> {
    library
        .get(primary)
        .or_else(|_| library.get(legacy))
        .map_err(|error| {
            anyhow!(
                "Required ABI symbol '{}' is missing: {}",
                String::from_utf8_lossy(primary),
                error
            )
        })
}

pub(crate) unsafe fn open_library(path: &Path) -> Result<libloading::Library> {
    #[cfg(windows)]
    {
        let flags = 0x00000008;
        libloading::os::windows::Library::load_with_flags(path, flags)
            .map(libloading::Library::from)
            .map_err(|error| anyhow!("Failed to load {}: {}", path.display(), error))
    }

    #[cfg(not(windows))]
    {
        libloading::Library::new(path)
            .map_err(|error| anyhow!("Failed to load {}: {}", path.display(), error))
    }
}
