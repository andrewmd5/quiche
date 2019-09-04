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
    if cfg!(target_os = "windows") {
        println!(
            "cargo:rustc-env=RAINWAY_RELEASE_URL={}",
            find_cargo_field("release_url")
        );
        println!(
            "cargo:rustc-env=RAINWAY_DOWNLOAD_FORMAT={}",
            find_cargo_field("downloaded_format")
        );
        println!(
            "cargo:rustc-env=MEDIA_PACK_URL={}",
            find_cargo_field("media_pack_url")
        );
         println!(
            "cargo:rustc-env=SENTRY_DNS={}",
            find_cargo_field("sentry_dns")
        );
        let mut res = winres::WindowsResource::new();
        res.set_manifest_file("manifest.xml");
        res.compile().unwrap();
    }
}
