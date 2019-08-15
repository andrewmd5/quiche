extern crate indicatif;
extern crate regex;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate winreg;

mod berror;
mod httpclient;
mod system;
mod utils;

use berror::BootstrapError;
use std::env;
use utils::ReleaseInfo;

fn main() -> Result<(), BootstrapError> {
    let system_info = match system::get_system_info() {
        Ok(s) => s,
        Err(error) => return Err(BootstrapError::new(error)),
    };
    if !system_info.is_x64 {
        return Err(BootstrapError::new(
            "Rainway is only supported by x64 operating systems.",
        ));
    }
    if !system_info.is_supported {
        return Err(BootstrapError::new(
            "Rainway is only supported on Windows 10 and Windows Server 2016+.",
        ));
    }
    if system_info.is_n_edition {
        match system::needs_media_pack() {
            Ok(needs_media_pack) => {
                if needs_media_pack {
                    return Err(BootstrapError::new(&format!(
                        "Please install the Windows Media Pack for {}",
                        system_info.product_name
                    )));
                }
            }
            Err(error) => return Err(BootstrapError::new(error)),
        };
    }
    match system::is_rainway_installed() {
        Ok(is_installed) => {
            if is_installed {
                return Err(BootstrapError::new(
                    "Rainway is already installed on this system.",
                ));
            }
        }
        Err(error) => return Err(BootstrapError::new(error)),
    };

    let release_info = match httpclient::download_json::<ReleaseInfo>(
        "https://releases.rainway.com/Installer_current.json",
    ) {
        Ok(s) => s,
        Err(error) => return Err(BootstrapError::new(&error)),
    };

    let install_url = format!(
        "https://releases.rainway.com/{}_{}.exe",
        release_info.name, release_info.version
    );

    let mut download_path = env::temp_dir();
    download_path.push(format!(
        "{}_{}.exe",
        release_info.name, release_info.version
    ));

    match httpclient::download_file(install_url.as_str(), &download_path) {
        Ok(f) => f,
        Err(error) => return Err(BootstrapError::new(&error)),
    };
    if utils::hash_file(&download_path).unwrap() != release_info.hash {
        return Err(BootstrapError::new(
            "Downloaded installer has an invalid signature.",
        ));
    }
    match system::run_intaller(&download_path) {
        Ok(f) => f,
        Err(error) => return Err(BootstrapError::new(&error)),
    };
    println!("Rainway installed!");
    Ok(())
}
