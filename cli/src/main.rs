use clap::{App, Arg};
use fern::colors::{Color, ColoredLevelConfig};
use quiche::bakery::Recipe;
use std::env;
use std::fs::File;
use std::path::Path;

fn setup_logging(verbosity: u64) -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .trace(Color::BrightCyan)
        .debug(Color::BrightMagenta)
        .warn(Color::BrightYellow)
        .info(Color::BrightGreen)
        .error(Color::BrightRed);

    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => base_config.level(log::LevelFilter::Debug),
        1 => base_config.level(log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Warn),
        _3_or_more => base_config.level(log::LevelFilter::Error),
    };

    // Separate file config so we can include colors in the terminal
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(File::create(format!("{}.log", env!("CARGO_PKG_NAME")))?);

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
fn main() {
    println!("{}", LOGO);

    let matches = App::new("Quiche CLI")
        .version("1.0")
        .author("Andrew Sampson <andrew@rainway.com>")
        .about("Build and fetch Rainway releases with ease.")
        .arg(
            Arg::with_name("recipe")
                .short("r")
                .long("recipe")
                .value_name("FILE")
                .required(true)
                .help("Sets a recipe for a release")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Increases logging verbosity each use for up to 3 times"),
        )
        .arg(
            Arg::with_name("release-override")
                .short("u")
                .value_name("URL")
                .help("Overrides the default release URL.")
                .takes_value(true),
        )
        .get_matches();

    let verbosity: u64 = matches.occurrences_of("verbose");
    setup_logging(verbosity).expect("failed to initialize logging.");

    log::debug!("checking for recipe file.");
    let recipe_path = match matches.value_of("recipe") {
        Some(s) => Path::new(s),
        None => panic!("The provided recipe path is empty."),
    };

    log::debug!("checking for release host override.");
    let release_override = matches.value_of("release-override").unwrap_or("");
    if !release_override.is_empty() {
        env::set_var("RELEASE_OVERRIDE", release_override);
        log::info!(
            "a release override was found. Quiche is now configured to use \"{}\" for remote releases.",
            release_override
        );
    }

    let mut recipe = Recipe::from(recipe_path);
    log::info!("read Quiche recipe from {}", recipe_path.display());
    if let Err(e) = recipe.prepare() {
        log::error!("the recipe ingredients could not be prepared. {}", e);
        panic!("preparations failed.");
    }
    log::info!("Quiche recipe prepared. attempted to bake.");

    let dinner = match recipe.bake() {
        Ok(d) => d,
        Err(e) => {
            log::error!("the recipe failed to bake properly. {}", e);
            panic!("bake failure.");
        }
    };

    if let Err(e) = recipe.stage(dinner) {
       log::error!("the recipe could not be staged. {}", e);
        panic!("stage failure.");
    }
    log::info!("dinner is served! the release was successfully baked.");
}

const LOGO: &str = r#"
  ____          _        _           
 / __ \        (_)      | |          
| |  | | _   _  _   ___ | |__    ___ 
| |  | || | | || | / __|| '_ \  / _ \
| |__| || |_| || || (__ | | | ||  __/
 \___\_\ \__,_||_| \___||_| |_| \___|   

"#;
