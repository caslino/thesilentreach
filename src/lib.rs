pub mod persistence;
pub mod player;
pub mod recorder;
pub mod universe;

use std::path::PathBuf;

/// Resolves the absolute path to the assets directory.
/// On macOS, it correctly identifies when running inside an .app bundle.
pub fn get_asset_root() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            let exe_str = exe_path.to_string_lossy();
            if exe_str.contains(".app/Contents/MacOS/") {
                let mut path = exe_path;
                path.pop(); // binary name
                path.pop(); // MacOS folder
                let bundle_assets = path.join("Resources").join("assets");
                if bundle_assets.exists() {
                    return bundle_assets;
                }
            }
        }
    }
    PathBuf::from("assets")
}
