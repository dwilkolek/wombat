use std::env;
fn main() {
    println!("cargo:rerun-if-env-changed=WOMBAT_API_URL");
    println!("cargo:rerun-if-env-changed=WOMBAT_API_USER");
    println!("cargo:rerun-if-env-changed=WOMBAT_API_PASSWORD");

    if !cfg!(debug_assertions) {
        env::var("WOMBAT_API_URL").expect("WOMBAT_API_URL must be set");
        env::var("WOMBAT_API_USER").expect("WOMBAT_API_USER must be set");
        env::var("WOMBAT_API_PASSWORD").expect("WOMBAT_API_PASSWORD must be set");
    }

    tauri_build::build();
}
