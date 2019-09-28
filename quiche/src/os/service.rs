use crate::etc::constants::BootstrapError;
use std::ffi::OsStr;
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

/// Checks if a service is installed on Windows via name.
pub fn service_exist(service_name: &str) -> bool {
    let manager_access = ServiceManagerAccess::CONNECT;
    if let Ok(service_manager) = ServiceManager::local_computer(None::<&str>, manager_access) {
        if let Ok(_service) =
            service_manager.open_service(service_name, ServiceAccess::QUERY_CONFIG)
        {
            return true;
        }
    }
    false
}

/// Starts a windows service by name.
pub fn start_service(service_name: &str) -> Result<bool, BootstrapError> {
    if !service_exist(service_name) {
        return Err(BootstrapError::ServiceMissing(service_name.to_string()));
    }
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
        Ok(sm) => sm,
        Err(_e) => return Err(BootstrapError::ServiceConnectionFailure),
    };
    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START | ServiceAccess::STOP;
    let service = match service_manager.open_service(service_name, service_access) {
        Ok(s) => s,
        Err(_e) => return Err(BootstrapError::ServiceOpenFailure),
    };
    let service_status = match service.query_status() {
        Ok(s) => s,
        Err(_e) => return Err(BootstrapError::ServiceQueryFailed),
    };
    if service_status.current_state != ServiceState::Stopped {
        if let Ok(_s) = service.stop() {
            log::info!("Stopped {}", service_name);
        }
    }
    match service.start(&[OsStr::new("Started from Rust!")]) {
        Ok(_o) => return Ok(true),
        Err(_e) => return Ok(false),
    }
}
