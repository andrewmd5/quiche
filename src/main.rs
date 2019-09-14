#![warn(rust_2018_idioms)]
//#![windows_subsystem = "windows"]

mod etc;
mod io;
mod net;
mod os;
mod ui;
mod updater;
use etc::constants::{is_compiled_for_64_bit, BootstrapError};
use etc::rainway::{
    error_on_duplicate_session, is_installed, is_outdated, kill_rainway_processes, launch_rainway, get_config_branch
};

use os::windows::{get_system_info, needs_media_pack};
use ui::messagebox::{show_error, show_error_with_url};
use ui::view::{apply_update, download_update, launch_and_close, verify_update};
use updater::{ActiveUpdate, UpdateType};

use web_view::*;

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

    kill_rainway_processes();

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
        update.update_type = UpdateType::Install;
    } else {
        update.update_type = UpdateType::Patch;
    }

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
    match updater::get_branch(get_config_branch()) {
        Some(b) => update.branch = b,
        None => {
            if rainway_installed {
                println!("Unable to check for latest branch. Starting current version.");
                launch_rainway();
                return Ok(());
            } else {
                let e = BootstrapError::ReleaseLookupFailed;
                show_error(caption, format!("{}", e));
                return Err(e);
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

    let webview = web_view::builder()
        .title("Rainway Boostrapper")
        .content(Content::Html(HTML))
        .size(800, 600)
        .resizable(false)
        .debug(false)
        .user_data(0)
        .invoke_handler(|_webview, arg| handler(_webview, arg, &update))
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

const HTML: &str = r#"
<!doctype html>
<html>
	<body>
		<p id="ticks"></p>
		<button onclick="external.invoke('download')">download</button>
		<button onclick="external.invoke('exit')">exit</button>
		<script type="text/javascript">
            function downloadProgress(v, total, downloaded) {
				document.getElementById('ticks').innerHTML = 'Download Progress for Rainway ' + v + '</br> Total Bytes: ' + total + '</br> Downloaded Bytes: ' + downloaded;
			}
            function downloadFailed(e) {
				document.getElementById('ticks').innerHTML = 'Download failed! ' + e;
			}
            function downloadComplete(e) {
				document.getElementById('ticks').innerHTML = 'Download Done! Verifying the update... ' + e;
                external.invoke('verify')
			}


            function verificationComplete(e) {
				document.getElementById('ticks').innerHTML = 'Verified the update! Installing...';
                 external.invoke('apply')

			}
            function verificationFailed(e) {
				document.getElementById('ticks').innerHTML = e;
			}

            function updateComplete(e) {
				document.getElementById('ticks').innerHTML = 'Rainway Installed/Updated! Closing...';
                setTimeout(
    function() {
      external.invoke('launch')
    }, 2500);

			}
            function updateFailed(e) {
				document.getElementById('ticks').innerHTML = e;
			}
            
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
