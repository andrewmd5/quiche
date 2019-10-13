use std::mem;
use std::path::PathBuf;
use winapi::shared::winerror::ERROR_NO_MORE_FILES;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use winapi::um::winbase::QueryFullProcessImageNameW;
use winapi::um::winnt::PROCESS_TERMINATE;

/// Basic Process Object.
#[derive(Debug)]
pub struct Process {
    id: u32,
    parent: u32,
    name: String,
    path: Option<PathBuf>,
}

impl Process {
    /// Get Process ID.
    pub fn id(&self) -> u32 {
        return self.id;
    }

    /// Get Parent Process ID.
    pub fn parent(&self) -> u32 {
        return self.parent;
    }

    /// Get Process Name.
    /// This value maybe program name.
    pub fn name(&self) -> &str {
        return self.name.as_str();
    }

    pub fn kill(&self) -> bool {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, self.id);
            if handle != INVALID_HANDLE_VALUE {
                return TerminateProcess(handle, 1) != 0 && CloseHandle(handle) != 0;
            }
            false
        }
    }

    /// Get Full file path of program if provided.
    pub fn path(&self) -> Option<PathBuf> {
        if let Some(path) = self.path.as_ref() {
            return Some(path.to_path_buf());
        } else {
            return None;
        }
    }
}

pub fn get_processes() -> Option<Vec<Process>> {
    let mut tasks: Vec<Process> = Vec::new();

    let snapshot = match snapshot() {
        Ok(s) => s,
        Err(_e) => {
            log::info!("{}", _e);
            return None;
        }
    };
    for p in snapshot {
        tasks.push(process(p));
    }
    Some(tasks)
}

fn process(entry: PROCESSENTRY32W) -> Process {
    let mut ps = Process {
        id: entry.th32ProcessID,
        parent: entry.th32ParentProcessID,
        name: String::from_utf16_lossy(&entry.szExeFile)
            .trim_end_matches(0x00 as char)
            .to_string(),
        path: None,
    };

    // Resolve Full Path.
    unsafe {
        // 0x00000400 = PROCESS_QUERY_INFORMATION
        let handle = OpenProcess(0x00000400, 0, entry.th32ProcessID);
        if handle != INVALID_HANDLE_VALUE {
            let mut name: [u16; 260] = [0; 260];
            let mut size = 260;
            let result = QueryFullProcessImageNameW(handle, 0, &mut name[0], &mut size);

            // Close Process Handle.
            CloseHandle(handle);

            // Check Result.
            if result != 0 {
                let full = PathBuf::from(
                    String::from_utf16_lossy(&name)
                        .trim_end_matches(0x00 as char)
                        .to_string(),
                );
                ps.path = Some(full);
                if ps.name.is_empty() && ps.path.is_some() {
                    ps.name = ps
                        .path()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                }
            }
        }
    }

    return ps;
}

fn snapshot() -> Result<Vec<PROCESSENTRY32W>, u32> {
    let mut processes: Vec<PROCESSENTRY32W> = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(GetLastError());
        }

        let mut entry: PROCESSENTRY32W = mem::uninitialized();
        entry.dwSize = mem::size_of_val(&entry) as _;

        // Get First Process.
        let mut result = Process32FirstW(snapshot, &mut entry);
        if result == 0 {
            CloseHandle(snapshot);
            let error = GetLastError();
            if error == ERROR_NO_MORE_FILES {
                return Ok(processes);
            } else {
                return Err(error);
            }
        } else {
            processes.push(entry.clone());
        }

        // Load Processes.
        loop {
            result = Process32NextW(snapshot, &mut entry);
            if result == 0 {
                CloseHandle(snapshot);
                let error = GetLastError();
                if error == ERROR_NO_MORE_FILES {
                    return Ok(processes);
                } else {
                    return Err(error);
                }
            }
            processes.push(entry.clone());
        }
    }
}
