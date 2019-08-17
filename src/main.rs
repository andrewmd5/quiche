extern crate indicatif;
extern crate regex;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate web_view;
extern crate winreg;

mod berror;
mod httpclient;
mod system;
mod utils;

use berror::BootstrapError;
use std::env;
use utils::ReleaseInfo;
use web_view::*;

fn main() -> Result<(), BootstrapError> {
    let system_info = system::get_system_info()?;

    if !system_info.is_x64 {
        return Err(BootstrapError::ArchitectureUnsupported);
    }
    if !system_info.is_supported {
        return Err(BootstrapError::WindowsVersionUnsupported);
    }

    if system_info.is_n_edition {
        if system::needs_media_pack()? {
            return Err(BootstrapError::NeedWindowsMediaPack);
        }
    }

    println!("{}", BootstrapError::ElevationRequired);

    if system::is_rainway_installed()? {
        return Err(BootstrapError::AlreadyInstalled);
    }

    let release_info = httpclient::download_json::<ReleaseInfo>(
        "https://releases.rainway.com/Installer_current.json",
    )?;

    let install_url = format!(
        "https://releases.rainway.com/{}_{}.exe",
        release_info.name, release_info.version
    );

    let mut download_path = env::temp_dir();
    download_path.push(format!(
        "{}_{}.exe",
        release_info.name, release_info.version
    ));

    httpclient::download_file(install_url.as_str(), &download_path)?;

    if utils::hash_file(&download_path).unwrap() != release_info.hash {
        return Err(BootstrapError::SignatureMismatch);
    }

    system::run_intaller(&download_path)?;

    println!("Rainway installed!");
    Ok(())
}
