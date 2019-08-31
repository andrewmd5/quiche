#![warn(rust_2018_idioms)]
//#![windows_subsystem = "windows"]

mod berror;
mod gui;
mod httpclient;
mod system;
mod utils;




use berror::BootstrapError;
use std::env;
use utils::ReleaseInfo;

fn main() -> Result<(), BootstrapError> {
    if !cfg!(debug_assertions) && utils::is_compiled_for_64_bit() {
        panic!("Buiild against i686-pc-windows-msvc for production releases.")
    }
    let caption = "Rainway Setup Error"; 
    match setup() {
        Ok(_) =>  return Ok(()),
        Err(e) => match e {
            BootstrapError::NeedWindowsMediaPack(_)  => gui::messagebox::show_error_with_url(caption, format!("{}", e), env!("MEDIA_PACK_URL")),
            _ =>  gui::messagebox::show_error(caption, format!("{}", e))
        }
    }
    Ok(())
}


fn setup() -> Result<(), BootstrapError> {
    let system_info = system::get_system_info()?;
    if !system_info.is_x64 {
        return Err(BootstrapError::ArchitectureUnsupported);
    }
    if !system_info.is_supported {
        return Err(BootstrapError::WindowsVersionUnsupported);
    }

    if system_info.is_n_edition {
        if system::needs_media_pack()? {
            return Err(BootstrapError::NeedWindowsMediaPack(
                system_info.product_name,
            ));
        }
    }

    if system::is_rainway_installed()? {
        return Err(BootstrapError::AlreadyInstalled);
    }

    let release_info = httpclient::download_json::<ReleaseInfo>(env!("RAINWAY_RELEASE_URL"))?;

    let install_url = format!(
        env!("RAINWAY_DOWNLOAD_FORMAT"),
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

    Ok(())
}