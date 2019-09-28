use clap::{App, Arg};
use console::style;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use quiche::io::disk::get_total_files;
use quiche::io::zip::zip_with_progress;
use quiche::updater::{get_releases, ReleaseBranch};
fn main() {
    println!("Commencing yak shaving");
    println!("{}", style(LOGO).cyan());

    let matches = App::new("Quiche CLI")
        .version("1.0")
        .author("Andrew Sampson <andrew@rainway.com>")
        .about("Build and fetch Rainway releases with ease.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("branch")
                .short("b")
                .value_name("NAME")
                .required(true)
                .help("Defines the branch quiche will be working work"),
        )
        .arg(
            Arg::with_name("version")
                .short("v")
                .value_name("VERSION")
                .required(true)
                .help("The version you wish to create or fetch"),
        ).get_matches();

    

    let branch = ReleaseBranch::from(matches.value_of("branch").unwrap_or(""));

    if let Some(releases) = get_releases() {
       
    } else {
         println!("cant");
    }

    let test_dir = String::from("E:\\UpdateTest\\InstalledFolder\\");
    let file_count = get_total_files(&test_dir).unwrap();

    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(200);
    bar.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("/|\\- ")
            .template("{spinner:.dim.bold} Packaging: {wide_msg}"),
    );
    let func_test = |file: String| {
        bar.set_message(format!("{}", file).as_str());
        bar.tick();
    };

    match zip_with_progress(
        test_dir,
        String::from("E:\\UpdateTest\\test.zip"),
        func_test,
    ) {
        Ok(f) => println!("Done"),
        Err(e) => println!("{}", e),
    };
    bar.finish_with_message("Done!");
}

const LOGO: &str = r#"
  ____          _        _           
 / __ \        (_)      | |          
| |  | | _   _  _   ___ | |__    ___ 
| |  | || | | || | / __|| '_ \  / _ \
| |__| || |_| || || (__ | | | ||  __/
 \___\_\ \__,_||_| \___||_| |_| \___|   

"#;
