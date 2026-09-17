/// Environment variable holding the hex-encoded Ed25519 public key used to verify OTA manifests.
/// It is read compile-time via `option_env!`, so it must be injected here rather than at runtime.
const PUBLIC_KEY_ENV: &str = "FIRMWARE_MANIFEST_PUBLIC_KEY_HEX";

/// Rejects a key that `main.rs` would silently treat as "no key configured". `option_env!` yields
/// `None` for an absent *or empty* variable, and `hex::decode` rejects anything that is not an even
/// number of hex digits, so both cases have to fail the build instead of producing firmware whose
/// OTA path can never succeed.
fn validate_public_key(raw: &str) {
    if raw.is_empty() {
        panic!(
            "{PUBLIC_KEY_ENV} is set but empty. Generate a key pair and export the 64-character \
             hex public key, or unset the variable to build an explicitly OTA-disabled image."
        );
    }
    if raw.len() != 64 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        panic!(
            "{PUBLIC_KEY_ENV} must be 64 hex characters (a 32-byte Ed25519 public key), got {} characters",
            raw.len()
        );
    }
}

fn main() {
    println!("cargo:rerun-if-changed=components/rlcd/CMakeLists.txt");
    println!("cargo:rerun-if-changed=components/rlcd/rlcd.c");
    println!("cargo:rerun-if-changed=components/rlcd/include/rlcd.h");

    if std::env::var_os("CARGO_FEATURE_ESP").is_some() {
        // Only the on-device build verifies manifests; host tests never apply an update.
        match std::env::var(PUBLIC_KEY_ENV) {
            Ok(raw) => {
                validate_public_key(&raw);
                println!("cargo:rustc-env={PUBLIC_KEY_ENV}={raw}");
                // Lets `main.rs` assert at compile time that verification is actually wired up.
                println!("cargo:rustc-cfg=firmware_ota_key");
            }
            Err(std::env::VarError::NotPresent) => {
                println!(
                    "cargo:warning={PUBLIC_KEY_ENV} is not set: building without OTA manifest \
                     verification. Every OTA job will be rejected as firmware_public_key_missing."
                );
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                panic!("{PUBLIC_KEY_ENV} is not valid UTF-8");
            }
        }
        embuild::espidf::sysenv::output();
    }
}
