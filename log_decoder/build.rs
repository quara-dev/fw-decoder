fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simplified build script without git dependencies
    println!("cargo:rustc-env=VERGEN_GIT_DESCRIBE=dev-build");
    Ok(())
}
