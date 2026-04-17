// Embedded static assets for production builds
// In debug mode, files are served from filesystem for hot reload
// In release mode, files are embedded in the binary

use rust_embed::RustEmbed;

/// Static assets embedded at compile time.
/// Only used in release builds for serving files from the binary.
#[derive(RustEmbed, Clone)]
#[folder = "static/"]
pub struct StaticAssets;
