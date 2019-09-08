use std::fs::{create_dir_all, File};
use std::io;
use std::path::PathBuf;

pub fn unzip(input: &PathBuf, output: &PathBuf) -> bool {
    let input_file = match File::open(&input) {
        Ok(f) => f,
        Err(_e) => return false,
    };
    let mut archive = match zip::ZipArchive::new(input_file) {
        Ok(a) => a,
        Err(_e) => return false,
    };
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_e) => return false,
        };
        let mut outpath = output.clone();
        outpath.push(file.sanitized_name());
        if (&*file.name()).ends_with('/') {
            println!(
                "File {} extracted to \"{}\"",
                i,
                outpath.as_path().display()
            );
            if let Err(_e) = create_dir_all(&outpath) {
                return false;
            }
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.as_path().display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    if let Err(_e) = create_dir_all(&p) {
                        return false;
                    }
                }
            }
            let mut outfile = match File::create(&outpath) {
                Ok(o) => o,
                Err(_e) => return false,
            };
            if let Err(_e) = io::copy(&mut file, &mut outfile) {
                return false;
            }
        }
    }
    true
}
