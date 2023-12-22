use std::process::{exit, Command};

fn main() -> Result<(), std::io::Error> {
    let maybe_modify = String::from(env!("BUILDSYS_MODIFY_IMAGE"));
    let ret = match maybe_modify.as_str() {
        "true" => Command::new("buildsys").arg("modify-image").status()?,
        _ => Command::new("buildsys").arg("build-variant").status()?,
    };
    if !ret.success() {
        exit(1);
    }
    Ok(())
}
