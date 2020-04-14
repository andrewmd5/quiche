const TOML: &'static str = include_str!("Cargo.toml");

pub fn find_cargo_field(key: &'static str) -> &'static str {
    if let Some(line) = TOML.split("\n").find(|x| x.starts_with(key)) {
        let start = line.find('"').unwrap() + 1;
        let end = line.rfind('"').unwrap();
        &line[start..end]
    } else {
        panic!("failed parsing version from cargo.toml");
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
        "cargo:rustc-env=BASE_RELEASE_URL={}",
        find_cargo_field("base_release_url")
    );
    println!(
        "cargo:rustc-env=UNINSTALL_KEY={}",
        find_cargo_field("uninstall_key")
    );
    println!(
        "cargo:rustc-env=RELEASE_PATH={}",
        find_cargo_field("release_path")
    );
    println!(
        "cargo:rustc-env=INSTALL_ENDPOINT={}",
        find_cargo_field("install_endpoint")
    );
    println!(
        "cargo:rustc-env=UPDATE_ENDPOINT={}",
        find_cargo_field("update_endpoint")
    );
    println!(
        "cargo:rustc-env=ACTIVATE_ENDPOINT={}",
        find_cargo_field("activate_endpoint")
    );
    // println!(
    //     "cargo:rustc-env=DEACTIVATE_ENDPOINT={}",
    //     find_cargo_field("deactivate_endpoint")
    // );
    println!(
        "cargo:rustc-env=API_ORIGIN={}",
        find_cargo_field("api_origin")
    );
    println!(
        "cargo:rustc-env=RAINWAY_KEY={}",
        find_cargo_field("rainway_key")
    );
}
