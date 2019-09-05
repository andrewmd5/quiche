#![warn(rust_2018_idioms)]
//#![windows_subsystem = "windows"]

mod etc;
mod io;
mod net;
mod os;
mod ui;
mod updater;

use etc::constants::{is_compiled_for_64_bit, BootstrapError};
use etc::rainway::{error_on_duplicate_session, is_installed, is_outdated};
use os::windows::{get_system_info, needs_media_pack};
use ui::callback::run_async;
use ui::messagebox::{show_error, show_error_with_url};
use updater::{ActiveUpdate, UpdateState};
use std::{
    sync::{Arc, RwLock},
    thread,
    env,
    time::Duration,
};


use serde::Deserialize;

use web_view::*;

#[derive(Debug, Copy, Clone)]
pub struct Progress {
    pub started: bool,
    pub len: u64,
    pub current: u64,
}

fn bridge<T: 'static>(webview: &mut WebView<'_, T>, arg: &str) -> WVResult {
    println!("INVOKED");
    match arg {
        "download" =>  println!("no pls.") /*setup_rainway(webview, "test".to_string(), "error".to_string())*/,
        "exit" => {
            //process::exit(0x0100);
        }
        _ => unimplemented!(),
    }
    Ok(())
}

fn main() -> Result<(), BootstrapError> {
    let caption = "Rainway Bootstrapper Error";
    let _guard = sentry::init(env!("SENTRY_DNS"));
    sentry::integrations::panic::register_panic_handler();
    if !cfg!(debug_assertions) && is_compiled_for_64_bit() {
        panic!("Build against i686-pc-windows-msvc for production releases.")
    }
    if let Err(e) = error_on_duplicate_session() {
        return Err(e);
    }
    let rainway_installed = match is_installed() {
        Ok(i) => i,
        Err(e) => {
            show_error(caption, format!("{}", e));
            sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
            return Err(e);
        }
    };

    let mut update = ActiveUpdate::default();
    if !rainway_installed {
        match check_system_compatibility() {
            Ok(go) => go,
            Err(e) => match e {
                BootstrapError::NeedWindowsMediaPack(_) => {
                    show_error_with_url(caption, format!("{}", e), env!("MEDIA_PACK_URL"));
                    sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
                    return Err(e);
                }
                _ => {
                    show_error(caption, format!("{}", e));
                    sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
                    return Err(e);
                }
            },
        }
    }
    //regardless of whether we need to update or install, we need the latest branch.
    match updater::get_branch(updater::ReleaseBranch::Stable) {
        Some(b) => update.branch = b,
        None => {
            let e = BootstrapError::ReleaseLookupFailed;
            show_error(caption, format!("{}", e));
            return Err(e);
        }
    }
    if !rainway_installed {
        match is_outdated(&update.branch.version) {
            Some(outdated) => {
                if !outdated {
                    println!("Shutting down because we're up-to-date.");
                    println!("TODO check if Rainway is running or not, launch if not.");
                    return Ok(());
                }
            }
            None => {
                println!("Shutting down because we failed to check if we are outdated.");
                return Ok(());
            }
        }
    }
    // if we're here, it means we need to update Rainway or install it.
    // TODO kill all rainway processes at this point, if they exist. 
    // TODO spawn the UI

    

    let webview = web_view::builder()
        .title("Rainway Boostrapper")
        .content(Content::Html(HTML))
        .size(800, 600)
        .resizable(false)
        .debug(false)
        .user_data(update)
        .invoke_handler(bridge)
        .build()?;

    webview.run()?;

    Ok(())
}

fn check_system_compatibility() -> Result<(), BootstrapError> {
    let system_info = get_system_info()?;
    if !system_info.is_x64 {
        return Err(BootstrapError::ArchitectureUnsupported);
    }

    if !system_info.is_supported {
        return Err(BootstrapError::WindowsVersionUnsupported);
    }

    if system_info.is_n_edition {
        if needs_media_pack()? {
            return Err(BootstrapError::NeedWindowsMediaPack(
                system_info.product_name,
            ));
        }
    }
    Ok(())
}

/*fn setup_rainway<T: 'static>(webview: &mut WebView<'_, T>, callback: String, error: String) {
    let handle = webview.handle();
    let user_data = webview.user_data_mut::<ActiveUpdate>();
    run_async(
        webview,
        move || {
           
            let arc = Arc::new(RwLock::new(user_data));
            let local_arc = arc.clone();
            let child = thread::spawn(move || loop {
                {
                    let my_rwlock = arc.clone();
                    let reader = my_rwlock.read().unwrap();
                    let f = *reader;
                    handle
                        .dispatch(move |webview| {
                            webview.eval(
                                format!(
                                    "updateTicks('Bytes to Download: {} -> Bytes Downloaded {}')",
                                    f.total_bytes, f.downloaded_bytes
                                )
                                .as_str(),
                            )
                        })
                        .unwrap();
                }
                thread::sleep(Duration::from_millis(100));
            });
            let fuck = download_file(local_arc, user_data.branch.manifest.unwrap().as_str(), &download_path).unwrap();
            let res = child.join();
            println!("test");
            get_system_info()
                .map_err(|err| format!("{}", err))
                .map(|output| format!("'{}'", "Done downloading!"))
        },
        callback,
        error,
    );
}*/

const HTML: &str = r#"
<!doctype html>
<html>
	<body>
		<p id="ticks"></p>
		<button onclick="external.invoke('download')">download</button>
		<button onclick="external.invoke('exit')">exit</button>
		<script type="text/javascript">
			function updateTicks(n) {
				document.getElementById('ticks').innerHTML = 'ticks ' + n;
			}
            function test(n) {
				document.getElementById('ticks').innerHTML = 'ticks ' + n;
			}
		</script>
	</body>
</html>
"#;
