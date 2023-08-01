use std::path::PathBuf;
use std::process::{exit, Command};

fn main() -> Result<(), std::io::Error> {
    let ca_bundle_path = PathBuf::from(env!("BUILDSYS_CACERTS_BUNDLE"));
    println!("cargo:rerun-if-changed={}", ca_bundle_path.display());
    let ret = Command::new("buildsys").arg("fetch-cacerts").status()?;
    if !ret.success() {
        exit(1);
    }
    Ok(())
}
