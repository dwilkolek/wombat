fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_protos();
    tauri_build::build();
    Ok(())
}

#[cfg(feature = "arh")]
fn build_protos() {
    println!("cargo:info=Compiling protos...");
    tonic_build::compile_protos("../arh/proto/arh.proto").unwrap();
}

#[cfg(not(feature = "arh"))]
fn build_protos() {
    println!("cargo:info=Skipping proto compilation");
}
