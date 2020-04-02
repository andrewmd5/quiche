use crate::os::guid::Guid;
use std::io::{Error, ErrorKind};
use std::mem::{MaybeUninit, size_of_val};

use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use winapi::shared::minwindef::DWORD;
use winapi::shared::winerror::{ERROR_MORE_DATA, ERROR_NO_MORE_FILES};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::minwinbase::STILL_ACTIVE;
use winapi::um::processthreadsapi::{
    GetCurrentProcessId, GetExitCodeProcess, OpenProcess, ProcessIdToSessionId, TerminateProcess,
};
use winapi::um::restartmanager::{
    RmEndSession, RmGetList, RmRebootReasonNone, RmRegisterResources, RmStartSession,
    RM_PROCESS_INFO,
};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

use winapi::um::winbase::QueryFullProcessImageNameW;
use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE};

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

    /// Retrieves the Remote Desktop Services session associated with a specified process.
    pub fn session_id(&self) -> u32 {
        unsafe {
            let mut session_id = 0;
            let r = ProcessIdToSessionId(self.id(), &mut session_id);
            if r != 0 {
                return session_id;
            }
            0
        }
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

pub fn get_current_process() -> Option<Process> {
    unsafe {
        let pid = GetCurrentProcessId();
        if let Some(processes) = get_processes() {
            for p in processes {
                if p.id() == pid {
                    return Some(p);
                }
            }
        }
        None
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

/// returns a list of processes that  have a particular file locked
pub fn get_procs_using_path<P: AsRef<Path>>(path: P) -> Result<Vec<Process>, Error> {
    let mut session_handle: DWORD = 0;
    let key = Guid::new().unwrap().format("N").unwrap();
    let mut s: Vec<_> = key.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let res = RmStartSession(&mut session_handle, 0, s.as_mut_ptr());
        if res != 0 {
            RmEndSession(session_handle);
            return Err(Error::from_raw_os_error(res as i32));
        }
        let wide_path: Vec<_> = path
            .as_ref()
            .to_str()
            .unwrap()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut resources = vec![wide_path.as_ptr()];
        if RmRegisterResources(
            session_handle,
            1,
            resources.as_mut_ptr(),
            0,
            null_mut(),
            0,
            null_mut(),
        ) != 0
        {
            RmEndSession(session_handle);
            return Err(Error::from(ErrorKind::ConnectionRefused));
        }

        let mut n_proc_info_needed = 0;
        let mut n_proc_info = 0;
        let mut reboot_reasons = RmRebootReasonNone;

        // Determine how much memory we need.
        let res = RmGetList(
            session_handle,
            &mut n_proc_info_needed,
            &mut n_proc_info,
            null_mut(),
            &mut reboot_reasons,
        );
        if res == 0 {
            RmEndSession(session_handle);
            return Ok(vec![]);
        }
        if res != ERROR_MORE_DATA {
            RmEndSession(session_handle);
            return Err(Error::from_raw_os_error(res as i32));
        }
        // Fetch the processes.
        let mut process_info: Vec<RM_PROCESS_INFO> =
            Vec::with_capacity(n_proc_info_needed as usize);
        n_proc_info = n_proc_info_needed;
        if RmGetList(
            session_handle,
            &mut n_proc_info_needed,
            &mut n_proc_info,
            process_info.as_mut_ptr(),
            &mut reboot_reasons,
        ) != 0
        {
            RmEndSession(session_handle);
            return Err(Error::from(ErrorKind::NotFound));
        }

        process_info.set_len(n_proc_info as usize);
        let mut ents: Vec<Process> = Vec::new();
        for info in process_info {
            ents.push(Process {
                parent: 0,
                id: info.Process.dwProcessId,
                path: None,
                name: String::from_utf16_lossy(
                    &info
                        .strAppName
                        .iter()
                        .map(|&v| v)
                        .take_while(|&c| c != 0x0000)
                        .map(|c| c)
                        .collect::<Vec<u16>>(),
                ),
            });
        }
        RmEndSession(session_handle);
        return Ok(ents);
    }
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

        let mut entry: PROCESSENTRY32W = MaybeUninit::uninit().assume_init();
        // We must set the size first or this will not work.
        entry.dwSize = size_of_val(&entry) as _;

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
