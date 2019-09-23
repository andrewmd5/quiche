use ansi_term::Colour::Green;
use clap::{App, Arg, SubCommand};
use std::io;

fn main() {
    println!("{}", Green.paint(LOGO));

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
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    let branch = matches.value_of("branch").unwrap_or("");


    println!("{}", branch);
}

const LOGO: &str = r#"
  ____          _        _           
 / __ \        (_)      | |          
| |  | | _   _  _   ___ | |__    ___ 
| |  | || | | || | / __|| '_ \  / _ \
| |__| || |_| || || (__ | | | ||  __/
 \___\_\ \__,_||_| \___||_| |_| \___|   

"#;
