#![warn(rust_2018_idioms)]
#![windows_subsystem = "windows"]

mod etc;
mod io;
mod net;
mod os;
mod ui;
mod updater;
use etc::constants::{is_compiled_for_64_bit, BootstrapError};
use etc::rainway::check_system_compatibility;
use etc::rainway::{
    error_on_duplicate_session, get_config_branch, is_installed, is_outdated,
    kill_rainway_processes, launch_rainway,
};

use ui::messagebox::{show_error, show_error_with_url};
use ui::view::{apply_update, download_update, launch_and_close, verify_update};
use ui::window::set_dpi_aware;
use updater::{ActiveUpdate, UpdateType};

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
    }

    if !rainway_installed {
        check_system_compatibility()?;
    }
    //regardless of whether we need to update or install, we need the latest branch.
    match updater::get_branch(get_config_branch()) {
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
        let outdated = match is_outdated(&update.branch.version, update.get_package_files()) {
            Some(o) => o,
            None => false,
        };
        if !outdated {
            println!("Rainway is not outdated, starting.");
            launch_rainway();
            return Ok(());
        }
    }

    set_dpi_aware();

    let index = Asset::get("index.html").unwrap();
    let html = std::str::from_utf8(&index).unwrap();

    let webview = web_view::builder()
        .title("Rainway Boostrapper")
        .content(Content::Html(html))
        .size(800, 600)
        .user_data(0)
        .resizable(false)
        .invoke_handler(|_webview, arg| handler(_webview, arg, &update))
        .build()?;

    webview.run()?;
    Ok(())
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
        _ => unimplemented!(),
    }
    Ok(())
}
