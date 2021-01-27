use crate::etc::constants::BootstrapError;
use std::ptr;

extern crate winapi;
extern crate windows_acl;

use windows_acl::acl::ACL;
use windows_acl::helper::string_to_sid;

use winapi::um::winnt;
use winapi::um::winnt::PSID;
use winapi::um::winsvc;

// CustomServiceManager is a simple clone of the windows_service service manager
// but we can acutally access the manager_handle in this case
pub struct CustomServiceManager {
    manager_handle: winsvc::SC_HANDLE,
}

impl CustomServiceManager {
    // create a new CustomServiceManager
    // This will always open a service manager for the current machine with all access
    pub fn new() -> Result<Self, BootstrapError> {
        let handle = unsafe {
            winsvc::OpenSCManagerW(ptr::null(), ptr::null(), winsvc::SC_MANAGER_ALL_ACCESS)
        };

        if handle.is_null() {
            Err(BootstrapError::ServiceConnectionFailure)
        } else {
            Ok(CustomServiceManager {
                manager_handle: handle,
            })
        }
    }

    // change_service_dacl will change the dacl for the given service
    // allowing it to be started, stopped and modified by any user on the system
    pub fn change_service_dacl(&self, service: &str) -> Result<(), BootstrapError> {
        // get the service
        let svc_handle = unsafe {
            winsvc::OpenServiceW(
                self.manager_handle,
                to_wstring(service).as_ptr(),
                winnt::READ_CONTROL | winnt::WRITE_DAC,
            )
        };

        if svc_handle.is_null() {
            // we coudlnt find the service then we dont want to do anything more
            log::info!("Unable to find service");
            return Err(BootstrapError::ServiceMissing(service.to_owned()));
        }

        // get the acl for the service
        let mut acl = match ACL::from_handle(
            svc_handle as winnt::HANDLE,
            winapi::um::accctrl::SE_SERVICE,
            false,
        ) {
            Ok(acl) => acl,
            Err(_) => return Err(BootstrapError::ServiceQueryFailed),
        };

        // S-1-1-0 is the SID for everyone
        let sid = if let Ok(sid) = string_to_sid("S-1-1-0") {
            sid
        } else {
            return Err(BootstrapError::NewSidFailed);
        };

        // now use that sid to allow all users some privledges
        match acl.allow(
            sid.as_ptr() as PSID,
            false,
            winsvc::SERVICE_START | winsvc::SERVICE_STOP | winnt::READ_CONTROL | winnt::DELETE
        ) {
            Ok(status) => {
                if !status {
                    Err(BootstrapError::SidUpdateFailed)
                } else {
                    Ok(())
                }
            }
            Err(_code) => {
                log::info!("Failed to update sid on service");
                Err(BootstrapError::SidUpdateFailed)
            }
        }
    }
}

impl Drop for CustomServiceManager {
    fn drop(&mut self) {
        unsafe {
            // make sure that we close the handle on this service when
            // we are done
            winsvc::CloseServiceHandle(self.manager_handle);
        }
    }
}

// helper so that we can call
// some winapi functions
fn to_wstring(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
