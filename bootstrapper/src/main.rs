#![warn(rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rainway;
mod ui;

use quiche::etc::constants::{BootstrapError, is_compiled_for_64_bit};
use rainway::{
    check_system_compatibility, error_on_duplicate_session, get_config_branch, is_installed, kill_rainway_processes, launch_rainway,
    get_install_path, get_installed_version
};

use quiche::updater::{ActiveUpdate, UpdateType, get_branch};
use ui::messagebox::{show_error, show_error_with_url};
use ui::view::{apply_update, download_update, launch_and_close, verify_update};
use ui::window::{set_dpi_aware};

use rust_embed::RustEmbed;
use web_view::{Content, WVResult, WebView};

#[derive(RustEmbed)]
#[folder = "resources/"]
struct Asset;

const CAPTION: &str = "Rainway Bootstrapper Error";

fn main() -> Result<(), BootstrapError> {
    let _guard = sentry::init(env!("SENTRY_DNS"));
    sentry::integrations::panic::register_panic_handler();
    if !cfg!(debug_assertions) && is_compiled_for_64_bit() {
        panic!("Build against i686-pc-windows-msvc for production releases.")
    }
    if let Err(e) = run() {
        match e {
            BootstrapError::NeedWindowsMediaPack(_) => {
                show_error_with_url(CAPTION, format!("{}", e), env!("MEDIA_PACK_URL"));
                panic!("{}", e);
            }
            _ => {
                show_error(CAPTION, format!("{}", e));
                panic!("{}", e);
            }
        }
    }
    Ok(())
}

fn run() -> Result<(), BootstrapError> {
    if let Err(e) = error_on_duplicate_session() {
        return Err(e);
    }

    kill_rainway_processes();

    let rainway_installed = is_installed()?;
    let mut update = ActiveUpdate::default();
    if !rainway_installed {
        update.update_type = UpdateType::Install;
    } else {
        update.update_type = UpdateType::Patch;
        update.current_version = match get_installed_version() {
            Some(v) => v,
            None => {
                launch_rainway();
                return Err(BootstrapError::LocalVersionMissing)
            },
        };
        update.install_path = match get_install_path() {
            Some(p) => p,
            None => {
                launch_rainway();
                return Err(BootstrapError::InstallPathMissing)
            },
        };
    }

    if !rainway_installed {
        check_system_compatibility()?;
    }
    //regardless of whether we need to update or install, we need the latest branch.
    match get_branch(get_config_branch()) {
        Some(b) => update.branch = b,
        None => {
            if rainway_installed {
                println!("Unable to check for latest branch. Starting current version.");
                launch_rainway();
                return Ok(());
            } else {
                return Err(BootstrapError::ReleaseLookupFailed);
            }
        }
    }
    update.temp_name = format!("{}{}", update.get_hash(), update.get_ext());
    //check if Rainway requires an update if it's installed
    if rainway_installed {
        let valid = update.validate();
        if valid {
            println!("Rainway is not outdated, starting.");
            launch_rainway();
            return Ok(());
        }
    }

    set_dpi_aware();

    let index = Asset::get("index.html").unwrap();
    let html = std::str::from_utf8(&index).unwrap();

    let webview = match web_view::builder()
        .title("Rainway Boostrapper")
        .content(Content::Html(html))
        .size(600, 380)
        .debug(true)
        .user_data(0)
        .resizable(false)
        .invoke_handler(|_webview, arg| handler(_webview, arg, &update))
        .build() {
            Ok(v) => v,
            Err(e) => return Err(BootstrapError::WebView(e.to_string()))
        };

    match webview.run() {
        Ok(_v) => return Ok(()),
        Err(e) => return Err(BootstrapError::WebView(e.to_string()))
    };
    
}

/// handles WebView external function calls
fn handler<T: 'static>(webview: &mut WebView<'_, T>, arg: &str, update: &ActiveUpdate) -> WVResult {
    match arg {
        "download" => {
            download_update(webview, update);
        }
        "verify" => {
            verify_update(webview, update);
        }
        "apply" => {
            apply_update(webview, update);
        }
        "launch" => {
            launch_and_close(webview);
        }
        "exit" => {
            std::process::exit(0);
        }
        _ => {
            if arg.contains("log|") {
                println!("[Javascript] {}", arg.split('|').collect::<Vec<&str>>()[1]);
            } else {
                unimplemented!()
            }
        }
    }
    Ok(())
}
