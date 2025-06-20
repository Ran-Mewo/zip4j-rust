//! Embedded library loading for bundled mode
//!
//! This module handles extracting and loading native libraries that are embedded
//! directly into the Rust binary when the "bundled" feature is enabled.


#[cfg(feature = "bundled")]
use once_cell::sync::Lazy;
#[cfg(feature = "bundled")]
use libloading::Library;
#[cfg(feature = "bundled")]
use tempfile::TempDir;

#[cfg(feature = "bundled")]
include!(concat!(env!("OUT_DIR"), "/embedded_libs.rs"));

#[cfg(feature = "bundled")]
static LIBRARY_LOADER: Lazy<LibraryLoader> = Lazy::new(|| {
    LibraryLoader::new().expect("Failed to initialize embedded library loader")
});

#[cfg(feature = "bundled")]
struct LibraryLoader {
    _temp_dir: TempDir,
    _library_path: std::path::PathBuf,
    library: Library,
}

#[cfg(feature = "bundled")]
impl LibraryLoader {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let platform = get_current_platform();

        let embedded_libs = get_embedded_libraries();
        let embedded_lib = embedded_libs.get(platform)
            .ok_or_else(|| format!("No embedded library found for platform: {}", platform))?;

        // Always extract to a temporary directory for clean isolation
        let temp_dir = tempfile::tempdir()?;
        let lib_path = temp_dir.path().join(embedded_lib.filename);
        std::fs::write(&lib_path, embedded_lib.data)?;

        // Set up environment so the dynamic linker can find the library
        #[cfg(target_os = "windows")]
        {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let temp_dir_str = temp_dir.path().to_string_lossy();
            let new_path = if current_path.is_empty() {
                temp_dir_str.to_string()
            } else {
                format!("{};{}", temp_dir_str, current_path)
            };
            std::env::set_var("PATH", new_path);
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let temp_dir_str = temp_dir.path().to_string_lossy();

            #[cfg(target_os = "linux")]
            {
                let current_path = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
                let new_path = if current_path.is_empty() {
                    temp_dir_str.to_string()
                } else {
                    format!("{}:{}", temp_dir_str, current_path)
                };
                std::env::set_var("LD_LIBRARY_PATH", new_path);
            }

            #[cfg(target_os = "macos")]
            {
                let current_path = std::env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
                let new_path = if current_path.is_empty() {
                    temp_dir_str.to_string()
                } else {
                    format!("{}:{}", temp_dir_str, current_path)
                };
                std::env::set_var("DYLD_LIBRARY_PATH", new_path);
            }
        }

        // Load the library
        let library = unsafe { Library::new(&lib_path)? };

        Ok(LibraryLoader {
            _temp_dir: temp_dir,
            _library_path: lib_path,
            library,
        })
    }

    fn library_path(&self) -> &std::path::Path {
        &self._library_path
    }

    fn get_symbol<T>(&self, symbol: &[u8]) -> Result<libloading::Symbol<T>, libloading::Error> {
        unsafe { self.library.get(symbol) }
    }
}

#[cfg(feature = "bundled")]
fn get_current_platform() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x86_64";

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    return "linux-x86_64";

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
    return "linux-x86_64-musl";

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "linux-aarch64";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "darwin-x86_64";

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "darwin-aarch64";

    // Fallback - this will cause an error at runtime if the platform isn't supported
    "unknown"
}

/// Get the path to the extracted library
#[cfg(feature = "bundled")]
pub fn get_library_path() -> &'static std::path::Path {
    LIBRARY_LOADER.library_path()
}

/// Initialize the embedded library loader (called automatically on first use)
#[cfg(feature = "bundled")]
pub fn initialize() -> Result<(), Box<dyn std::error::Error>> {
    // This will trigger the lazy initialization
    Lazy::force(&LIBRARY_LOADER);
    Ok(())
}

/// Ensure the embedded library is initialized, converting errors to ZipError
#[cfg(feature = "bundled")]
pub fn ensure_initialized() -> crate::error::Result<()> {
    initialize().map_err(|e| crate::error::ZipError::Unknown(format!("Failed to initialize embedded library: {}", e)))
}

/// Get a function pointer from the embedded library
#[cfg(feature = "bundled")]
pub fn get_function<T>(symbol: &[u8]) -> Result<libloading::Symbol<T>, Box<dyn std::error::Error>> {
    Ok(LIBRARY_LOADER.get_symbol(symbol)?)
}

// Stub implementations for when bundled feature is not enabled
#[cfg(not(feature = "bundled"))]
pub fn get_function<T>(_symbol: &[u8]) -> Result<libloading::Symbol<T>, Box<dyn std::error::Error>> {
    Err("Bundled feature not enabled".into())
}

#[cfg(not(feature = "bundled"))]
pub fn initialize() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
