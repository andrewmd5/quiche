extern crate winres;

const TOML: &'static str = include_str!("Cargo.toml");

pub fn find_cargo_field(key: &'static str) -> &'static str {
    if let Some(line) = TOML.split("\n").find(|x| x.starts_with(key)) {
        let start = line.find('"').unwrap() + 1;
        let end = line.rfind('"').unwrap();
        &line[start..end]
    } else {
        panic!("failed parsing version from Cagro.toml");
    }
}

fn main() {
    if !cfg!(target_os = "windows") {
        panic!("Only Windows builds are supported.");
    }
    let profile = std::env::var("PROFILE").unwrap();
    match profile.as_str() {
        "release" => {
            let target = std::env::var("TARGET").unwrap();
            if target != "i686-pc-windows-msvc" {
                panic!("Build against i686-pc-windows-msvc for production releases. Only x32 is supported.");
            }
        }
        _ => (),
    }
    println!(
        "cargo:rustc-env=RAINWAY_SERVICE={}",
        find_cargo_field("service_name")
    );
    println!(
        "cargo:rustc-env=MEDIA_PACK_URL={}",
        find_cargo_field("media_pack_url")
    );
    println!(
        "cargo:rustc-env=DOTNET_URL={}",
        find_cargo_field("dotnet_framework_url")
    );
    println!(
        "cargo:rustc-env=SENTRY_DNS={}",
        find_cargo_field("sentry_dns")
    );
    let mut res = winres::WindowsResource::new();
    res.set_icon("../resources/ProgramIcon.ico");
    res.set_manifest_file("manifest.xml");
    res.compile().unwrap();
}
