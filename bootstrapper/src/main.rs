#![warn(rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rainway;
mod ui;

use log::Level;
use quiche::etc::constants::{is_compiled_for_64_bit, BootstrapError};
use quiche::io::ico::IconDir;
use quiche::os::windows::detach_rdp_session;
use quiche::os::windows::{is_elevated, is_run_as_admin};
use quiche::updater::{is_installed, ActiveUpdate, UpdateType};
use rainway::{
    check_system_compatibility, error_on_duplicate_session, kill_rainway, launch_rainway,
};
use rust_embed::RustEmbed;
use ui::native::{show_error, show_error_with_url, try_elevate, local_app_data};
use ui::view::{apply_update, download_update, launch_and_close, verify_update};
use web_view::{Content, Icon, WVResult, WebView};
#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR\\..\\resources"]
struct Asset;

struct Resources {
    html: String,
    icon: Icon,
}

const CAPTION: &str = "Rainway Bootstrapper Error";

fn main() -> Result<(), BootstrapError> {
    let _guard = sentry::init(env!("SENTRY_DNS"));
    sentry::integrations::panic::register_panic_handler();
    if !cfg!(debug_assertions) && is_compiled_for_64_bit() {
        panic!("Build against i686-pc-windows-msvc for production releases.")
    }
    let verbosity = if !cfg!(debug_assertions) { 1 } else { 0 };

    setup_logging(verbosity).expect("failed to initialize logging.");

    // If we get an uninstall argument then do that and exit straight up
    // without spinning up anything else
    let mut args = std::env::args();
    if args.any(|x| x == "uninstall") {
        post_uninstall();
        return Ok(());
    }

    if let Err(e) = run() {
        match e {
            BootstrapError::NeedWindowsMediaPack(_) => {
                show_error_with_url(CAPTION, format!("{}", e), env!("MEDIA_PACK_URL"));
                panic!("{}", e);
            }
            BootstrapError::NeedDotNetFramework => {
                show_error_with_url(CAPTION, format!("{}", e), env!("DOTNET_URL"));
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
        log::error!("found another bootstrapper session. killing session.");
        return Err(e);
    }

    if detach_rdp_session() {
        log::info!("Session was detached");
    } else {
        log::info!("Failed or not need to detach session.");
    }

    kill_rainway();

    let mut update = ActiveUpdate::default();
    let rainway_installed = is_installed()?;
    log::info!("Rainway installed: {}", rainway_installed);
    if !rainway_installed {
        update.update_type = UpdateType::Install;
    } else {
        if let Err(e) = update.store_install_info() {
            launch_rainway(&update.install_info.path);
            return Err(e);
        }
        let should_self_update = match update.should_self_update() {
            Ok(_) => true,
            Err(_) =>  false
        };
        if should_self_update {
            if !is_elevated() || !is_run_as_admin() {
                log::warn!("elevated: {}", try_elevate());
                std::process::exit(0);
            }
            if let Err(e) = update.start_self_update() {
                log::error!("could not self-update the bootstrapper. {}", e);
            }
        }
        update.update_type = UpdateType::Patch;
    }

    log::info!("update type: {}", update.update_type);

    if !rainway_installed {
        log::info!("checking system compatibility.");
        check_system_compatibility()?;
    }

    //regardless of whether we need to update or install, we need the latest branch.
    let config_branch = update.install_info.branch;
    log::info!("user branch: {}", config_branch);
    if let Err(e) = update.get_manifest(config_branch) {
        if rainway_installed {
            log::error!("unable to check for latest branch. starting currently installed version.");
            launch_rainway(&update.install_info.path);
            sentry::capture_message(
                format!("Failed to fetch branch {}. {}", config_branch, e).as_str(),
                sentry::Level::Error,
            );
            return Ok(());
        } else {
            return Err(e);
        }
    }
    update.set_temp_file();
    log::debug!("temp file name: {}", update.temp_name);

    //check if Rainway requires an update if it's installed
    if rainway_installed {
        log::info!("validating Rainway installation.");
        let valid = update.validate();
        if valid {
            log::info!("Rainway is not outdated, starting.");
            launch_rainway(&update.install_info.path);
            return Ok(());
        }
        log::warn!("the current Rainway installation requires an update.");
    }

    // at this point we know we need to either update or install
    // so if we are not already elevated then go elevate ourselves
    if !is_elevated() || !is_run_as_admin() {
        log::warn!("elevated: {}", try_elevate());
        std::process::exit(0);
    }

    let resources = load_resources()?;

    let mut webview = match web_view::builder()
        .title("Rainway Setup")
        .content(Content::Html(resources.html))
        .size(600, 380)
        .debug(true)
        .user_data(0)
        .frameless(true)
        .resizable(false)
        .invoke_handler(|_webview, arg| handler(_webview, arg, &update))
        .build()
    {
        Ok(v) => v,
        Err(e) => return Err(BootstrapError::WebView(e.to_string())),
    };

    if resources.icon.length > 0 {
        webview.set_icon(resources.icon);
    }

    match webview.run() {
        Ok(_v) => return Ok(()),
        Err(e) => return Err(BootstrapError::WebView(e.to_string())),
    };
}

/// handles loading our bundled application resources.
fn load_resources() -> Result<Resources, BootstrapError> {
    let source = match Asset::get("index.html") {
        Some(r) => r,
        None => {
            return Err(BootstrapError::ResourceLoadError(
                "Could locate UI source.".to_string(),
            ))
        }
    };
    let html = std::str::from_utf8(&source)?;
    if html.is_empty() {
        return Err(BootstrapError::ResourceLoadError(
            "The HTML source is empty.".to_string(),
        ));
    }

    let mut icon_resource = Icon::default();
    if let Some(i) = Asset::get("ProgramIcon.ico") {
        let icon = i.into_owned();
        let mut icon_dir = IconDir::from(&icon)?;
        // there is a bug in the WINAPI where it uses the lowest resolution possible
        // if the resolution is uncommon (such as 142x142), so we remove all other options.
        icon_dir.filter_for(64, 64);
        if let Some(entry) = icon_dir.entries.first() {
            // encode our icon directory again with our new single entry
            let icon_data = icon_dir.encode()?;
            let data_length = icon_data.len() as u32;
            icon_resource = Icon {
                data: icon_data,
                width: entry.width as u32,
                height: entry.height as u32,
                length: data_length,
            };
        }
    }
    Ok(Resources {
        icon: icon_resource,
        html: html.to_string(),
    })
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
            if update.update_type != UpdateType::Install {
                launch_and_close(webview, update);
            } else {
                std::process::exit(0);
            }
        }
        "minimize" => {
            webview.minimize();
        }
        "exit" => {
            std::process::exit(0);
        }
        _ => {
            if arg.contains("log|") {
                log::debug!("[Javascript] {}", arg.split('|').collect::<Vec<&str>>()[1]);
            } else {
                log::warn!("{}", arg);
                unimplemented!()
            }
        }
    }
    Ok(())
}

fn setup_logging(verbosity: u64) -> Result<(), fern::InitError> {
    use fern::colors::{Color, ColoredLevelConfig};
    use std::fs::File;
    let colors = ColoredLevelConfig::new()
        .trace(Color::BrightCyan)
        .debug(Color::BrightMagenta)
        .warn(Color::BrightYellow)
        .info(Color::BrightGreen)
        .error(Color::BrightRed);

    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => base_config
            .level(log::LevelFilter::Debug)
            .level_for("hyper", log::LevelFilter::Info)
            .level_for("tokio_reactor", log::LevelFilter::Info),
        1 => base_config.level(log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Warn),
        _3_or_more => base_config.level(log::LevelFilter::Error),
    };

    // Separate file config so we can include colors in the terminal
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            add_breadcrumb(message.to_string(), record.level());
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(File::create(format!("{}\\Rainway\\Dashboard\\{}.log", local_app_data().unwrap().into_os_string().into_string().unwrap(), env!("CARGO_PKG_NAME")))?);

    let stdout_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}

fn add_breadcrumb(message: String, level: Level) {
    let sentry_level = match level {
        Level::Debug => sentry::Level::Debug,
        Level::Error => sentry::Level::Error,
        Level::Info => sentry::Level::Info,
        Level::Warn => sentry::Level::Warning,
        Level::Trace => sentry::Level::Debug,
    };
    sentry::add_breadcrumb(|| sentry::Breadcrumb {
        ty: "log".into(),
        level: sentry_level,
        message: Some(message.into()),
        ..Default::default()
    });
}

fn post_uninstall() {
    // Find our install key
    use quiche::updater;
    if let Ok(rainway_app) = updater::get_rainway_key() {
        if let Ok(install_info) = ActiveUpdate::get_install_info() {
            // Now we have all the info we want to send
            ActiveUpdate::store_event(updater::RainwayAppState::Deactivate);

            updater::post_deactivate(rainway_app.setup_id, install_info.version);
        }
    }
}
