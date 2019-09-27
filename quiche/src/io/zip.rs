use std::fs::{create_dir_all, File};
use std::io::{copy, Error, ErrorKind};
use std::path::{Path, PathBuf};

use crate::io::disk::get_dir_files;
use std::io::prelude::*;
use zip::result::ZipError;
use zip::write::FileOptions;

/// creates a zip file from a given directory
pub fn zip_with_progress<F>(input: String, output: String, callback: F) -> Result<(), Error>
where
    F: Fn(String),
{
    if !Path::new(&input).is_dir() {
        return Err(Error::from(ZipError::FileNotFound));
    }
    let path = Path::new(&output);
    let output_file = File::create(&path)?;
    let files = match get_dir_files(&input) {
        Some(f) => f,
        None => return Err(Error::from(ErrorKind::NotFound)),
    };
    let mut zip = zip::ZipWriter::new(output_file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Bzip2);
    let mut buffer = Vec::new();

    for entry in files {
        callback(entry.clone());
        let mut file_path = PathBuf::new();
        file_path.push(&input);
        file_path.push(&entry);
        zip.start_file_from_path(Path::new(&entry), options)?;
        let mut f = File::open(&file_path)?;
        f.read_to_end(&mut buffer)?;
        zip.write_all(&*buffer)?;
        buffer.clear();
    }
    zip.finish()?;
    Ok(())
}

/// unzips an archive to a target directory,
/// returning false if any files false.
pub fn unzip(input: &PathBuf, output: &PathBuf) -> Result<(), Error> {
    let input_file = File::open(&input)?;
    let mut archive = zip::ZipArchive::new(input_file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut outpath = output.clone();
        outpath.push(file.sanitized_name());
        if (&*file.name()).ends_with('/') {
            #[cfg(debug_assertions)]
            println!(
                "File {} extracted to \"{}\"",
                i,
                outpath.as_path().display()
            );
            create_dir_all(&outpath)?;
        } else {
            #[cfg(debug_assertions)]
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.as_path().display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    create_dir_all(&p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}
