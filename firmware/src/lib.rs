#[cfg(feature = "esp")]
pub mod buttons;
pub mod config;
pub mod display;
pub mod enrollment;
#[cfg(feature = "esp")]
pub mod esp_config;
#[cfg(feature = "esp")]
pub mod esp_enrollment;
#[cfg(feature = "esp")]
pub mod esp_mqtt;
#[cfg(feature = "esp")]
pub mod esp_storage;
pub mod flash_cache;
pub mod local_screen;
pub mod mqtt;
pub mod page_controller;
pub mod pages;
pub mod power;
pub mod provisioning;
#[cfg(feature = "esp")]
pub mod provisioning_esp;
pub mod release_sync;
#[cfg(feature = "esp")]
pub mod rlcd;
pub mod runtime;

pub const MAX_DISPLAY_RELEASE_BYTES: usize = 2 * 1024 * 1024;
pub const SUPPORTED_DISPLAY_DOCUMENT_VERSION: u16 = 1;
pub const DISPLAY_PHYSICAL_WIDTH: u16 = 400;
pub const DISPLAY_PHYSICAL_HEIGHT: u16 = 300;
