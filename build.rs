fn main() {
    println!("cargo:rerun-if-changed=components/rlcd/CMakeLists.txt");
    println!("cargo:rerun-if-changed=components/rlcd/rlcd.c");
    println!("cargo:rerun-if-changed=components/rlcd/include/rlcd.h");

    if std::env::var_os("CARGO_FEATURE_ESP").is_some() {
        embuild::espidf::sysenv::output();
    }
}
