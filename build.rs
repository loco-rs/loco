#[cfg(feature = "embedded_assets")]
use std::{env, path::Path};

fn main() {
    #[cfg(feature = "embedded_assets")]
    embedded_assets_main();

    #[cfg(not(feature = "embedded_assets"))]
    {
        // No-op when feature is disabled
    }
}

#[cfg(feature = "embedded_assets")]
fn embedded_assets_main() {
    // Import the embedded_assets module from the build directory
    #[path = "build/embedded_assets.rs"]
    mod embedded_assets;
    use embedded_assets::build_static_assets;

    // Get OUT_DIR environment variable - this is required for build scripts
    let out_dir = env::var("OUT_DIR").unwrap_or_else(|e| {
        // This should trigger a build failure
        panic!("OUT_DIR environment variable not set: {e}");
    });

    // Convert to a path
    let out_dir_path = Path::new(&out_dir);

    // Call the build_static_assets function with the OUT_DIR
    build_static_assets(out_dir_path);
}
