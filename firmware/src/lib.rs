pub mod config;
pub mod display;
pub mod mqtt;
pub mod provisioning;
pub mod runtime;

pub const MAX_DISPLAY_RELEASE_BYTES: usize = 2 * 1024 * 1024;
pub const SUPPORTED_DISPLAY_DOCUMENT_VERSION: u16 = 1;
