#![warn(rust_2018_idioms)]
//#![windows_subsystem = "windows"]

mod berror;
mod httpclient;
mod system;
mod utils;
mod callback;


use berror::BootstrapError;
use std::env;
use utils::ReleaseInfo;
use web_view::*;
use std::process;

fn main() -> Result<(), BootstrapError> {
    if !cfg!(debug_assertions) && utils::is_compiled_for_64_bit() {
        panic!("Buiild against i686-pc-windows-msvc for production releases.")
    }

    let webview = web_view::builder()
        .title("Rainway Boostrapper")
        .content(Content::Html(HTML))
        .size(800, 600)
        .resizable(false)
        .debug(true)
        .user_data(0)
        .invoke_handler(invoke_handler)
        .build()?;

    webview.run()?;

    //setup()?;

    Ok(())
}

//TODO break each call out into an async call so we can produce results on the UI
fn setup_rainway<T: 'static>(webview: &mut WebView<T>, callback: String, error: String)  {
      callback::run_async(webview, || {
        system::get_system_info().map_err(|err| format!("{}", err))
            .map(|output| format!("'{}'", output.product_name))
        },
        callback, error
    );
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

fn invoke_handler<T: 'static>(webview: &mut WebView<'_, T>, arg: &str) -> WVResult {
    println!("INVOKED");
    match arg {
        "test" => {
           setup_rainway(webview, "test".to_string(), "error".to_string())
        },
        "exit" => {
            process::exit(0x0100);
        }
        _ => unimplemented!(),
    }
    Ok(())
}

const HTML: &str = r#"
<!doctype html>
<html>
	<body>
		<p id="output"></p>
		<button onclick="external.invoke('test')">Call Rust</button>
		<button onclick="external.invoke('exit')">Exit</button>
		<script type="text/javascript">
			function test(v) {
				document.getElementById('output').innerHTML = 'Result From Rust: ' + v;
			}
            function error(e) {
				document.getElementById('output').innerHTML = 'Error From Rust: ' + e;
			}
		</script>
	</body>
</html>
"#;
