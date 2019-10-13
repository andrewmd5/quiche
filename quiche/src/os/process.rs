use std::mem;
use std::path::PathBuf;
use winapi::shared::winerror::{ERROR_NO_MORE_FILES};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess, GetExitCodeProcess};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use winapi::um::winbase::QueryFullProcessImageNameW;
use winapi::um::winnt::{PROCESS_TERMINATE, PROCESS_QUERY_INFORMATION};
use winapi::um::minwinbase::STILL_ACTIVE;

/// Basic Process Object.
#[derive(Debug)]
pub struct Process {
    id: u32,
    parent: u32,
    name: String,
    path: Option<PathBuf>,
}

impl Process {
    /// Get the underlying process id
    pub fn id(&self) -> u32 {
        return self.id;
    }

    /// Gets the immediate parent process id
    pub fn parent(&self) -> u32 {
        return self.parent;
    }

    /// Gets the process name.
    pub fn name(&self) -> &str {
        return self.name.as_str();
    }

    /// Kills the underlying process with prodigious. 
    pub fn kill(&self) -> bool {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, self.id);
            if handle != INVALID_HANDLE_VALUE {
                return TerminateProcess(handle, 1) != 0 && CloseHandle(handle) != 0;
            }
            false
        }
    }

    /// Determines if the underlying process is still running by using GetExitCodeProcess. 
    /// It will return STILL_ACTIVE (259) if the process is still running. 
    /// Microsoft says people should NOT use 259 as an exit code, so this should be fine.
    pub fn is_running(&self) -> bool {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, self.id);
            if handle == INVALID_HANDLE_VALUE {
                return false;
            }
            let mut exit_code = 0;
            let ret = GetExitCodeProcess(handle, &mut exit_code);
            if ret == 0 {
                return false;
            }
            CloseHandle(handle);
            return exit_code == STILL_ACTIVE;
        }
    }

    /// Gets the full file path of program if provided.
    pub fn path(&self) -> Option<PathBuf> {
        if let Some(path) = self.path.as_ref() {
            return Some(path.to_path_buf());
        } else {
            return None;
        }
    }
}
/// Returns a list of processes running on the system. 
/// will return `None` if there was an issue generating a snapshot from the Windows API
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
/// Turns a `PROCESSENTRY32W` structure into a `Process` object. 
fn process(entry: PROCESSENTRY32W) -> Process {
    let mut ps = Process {
        id: entry.th32ProcessID,
        parent: entry.th32ParentProcessID,
        name: String::from_utf16_lossy(&entry.szExeFile)
            .trim_end_matches(0x00 as char)
            .to_string(),
        path: None,
    };

    // Resolve the actual app name/path
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, entry.th32ProcessID);
        if handle != INVALID_HANDLE_VALUE {
            let mut name: [u16; 260] = [0; 260];
            let mut size = 260;
            let result = QueryFullProcessImageNameW(handle, 0, &mut name[0], &mut size);

            // Close process Handle.
            CloseHandle(handle);
            // win api functions return zero if they failed.
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

/// Queries for a list of active processes on the system. 
/// Everyone seems to have forgotten that 32-bit processes cannot access 64-bit process modules.
/// So every "solution" for getting system proceses in most languages is literally a case of "works on my machine."
/// In their defense, 32-bit shouldn't be your default target anymore, but in our case we need the process name of 64-bit apps.
/// To achieve this I wrote my own snapshot implementation which avoids module access. 
fn snapshot() -> Result<Vec<PROCESSENTRY32W>, u32> {
    let mut processes: Vec<PROCESSENTRY32W> = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(GetLastError());
        }

        let mut entry: PROCESSENTRY32W = mem::uninitialized();
        // We must set the size first or this will not work.
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

        // Now loop over all the others.
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
