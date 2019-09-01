use crate::utils::hash_file;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Default)]
struct FileInfo {
    pub path: String,
    pub hash: String,
}

pub fn create_snapshot_manifest(previous_version: &str, new_version: &str) {
    let old_path = Path::new(previous_version);
    let new_path = Path::new(new_version);

    let old_files = visit_dirs(old_path).unwrap();
    let new_files = visit_dirs(new_path).unwrap();

    let deleted_files: Vec<FileInfo> = old_files
        .into_iter()
        .filter(|x| !new_files.clone().into_iter().any(|u| u.path == x.path))
        .collect();

    for df in deleted_files {
        println!("{}", df.path);
    }
}

fn visit_dirs(dir: &Path) -> Option<Vec<FileInfo>> {
    if dir.is_dir() {
        let mut files: Vec<FileInfo> = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if let Some(mut f) = visit_dirs(&path) {
                    files.append(&mut f);
                }
            } else {
                files.push(FileInfo {
                    path: String::from(path.as_path().to_str().unwrap()),
                    hash: hash_file(&path).unwrap(),
                });
            }
        }
        return Some(files);
    }
    None
}
