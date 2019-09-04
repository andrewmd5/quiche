#![warn(rust_2018_idioms)]
//#![windows_subsystem = "windows"]

mod etc;
mod io;
mod net;
mod os;
mod ui;

use etc::constants::{is_compiled_for_64_bit, BootstrapError};
use etc::rainway::{error_on_duplicate_session, is_installed};
use os::windows::{get_system_info, needs_media_pack};
use ui::messagebox::{show_error, show_error_with_url};

use serde::Deserialize;

use web_view::*;

#[derive(Debug, Copy, Clone)]
pub struct Progress {
    pub started: bool,
    pub len: u64,
    pub current: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
/// Release info is pulled from a remote JSON config [here](https://releases.rainway.com/Installer_current.json).
/// The information located inside that config can be used to form a download URL.
pub struct ReleaseInfo {
    /// The prefix on our installer.
    pub name: String,
    /// The current release version.
    pub version: String,
    /// The SHA256 hash of the installer.
    /// Used to validate if the file downloaded properly.
    pub hash: String,
}


fn bridge<T: 'static>(webview: &mut WebView<'_, T>, arg: &str) -> WVResult {
    println!("INVOKED");
    match arg {
        "download" => println!("cool")/*setup_rainway(webview, "test".to_string(), "error".to_string())*/,
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
        //TODO fetch latest release.
        println!("System good!");
    }

    let webview = web_view::builder()
        .title("Rainway Boostrapper")
        .content(Content::Html(HTML))
        .size(800, 600)
        .resizable(false)
        .debug(false)
        .user_data(0)
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
    callback::run_async(
        webview,
        move || {
            let arc = Arc::new(RwLock::new(Progress {
                started: false,
                len: 0,
                current: 0,
            }));
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
                                    f.len, f.current
                                )
                                .as_str(),
                            )
                        })
                        .unwrap();
                    if f.started && f.len == f.current {
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(100));
            });
            let release_info = download_json::<ReleaseInfo>(env!("RAINWAY_RELEASE_URL")).unwrap();
            let install_url = format!(
                env!("RAINWAY_DOWNLOAD_FORMAT"),
                release_info.name, release_info.version
            );
            let mut download_path = env::temp_dir();
            download_path.push(format!(
                "{}_{}.exe",
                release_info.name, release_info.version
            ));
            let fuck = download_file_r(local_arc, install_url.as_str(), &download_path).unwrap();
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
