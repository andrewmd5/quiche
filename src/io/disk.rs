use fs_extra::dir::get_dir_content;
use std::ffi::OsStr;
use std::fs::{self, ReadDir};
use std::io::Error;
use std::path::{Path, PathBuf};

pub fn dir_contains_all_files(files: Vec<String>, input: &String) -> bool {
    let input_path = Path::new(input);
    if !input_path.exists() {
        return false;
    }
    let dir_content = get_dir_content(input_path).unwrap();
    let dir_files: Vec<String> = (&dir_content.files)
        .into_iter()
        .map(|file| file.replace(input, ""))
        .collect();
    for file in files {
        if !(&dir_files).into_iter().any(|v| v.clone() == file) {
            println!("{} is missing from the applied update.", file);
            return false;
        }
    }
    true
}

fn get_filename(path: &PathBuf) -> String {
    let ext = String::from(
        path.extension()
            .unwrap_or(OsStr::new(""))
            .to_str()
            .unwrap_or_default(),
    );
    let stem = String::from(
        path.file_stem()
            .unwrap_or(OsStr::new(""))
            .to_str()
            .unwrap_or_default(),
    );
    let dot = match ext.is_empty() {
        true => "",
        false => ".",
    };
    format!("{}{}{}", stem, dot, ext)
}

pub fn delete_dir_contents(read_dir_res: Result<ReadDir, Error>, ignored: Vec<String>) {
    if let Ok(dir) = read_dir_res {
        for entry in dir {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    fs::remove_dir_all(path).expect("Failed to remove a dir");
                } else {
                    if (&ignored)
                        .into_iter()
                        .any(|v| v.clone() == get_filename(&path))
                    {
                        println!("Skipping {}", path.display());
                        continue;
                    }
                    fs::remove_file(path).expect("Failed to remove a file");
                }
            };
        }
    };
}
