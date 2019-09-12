use crate::etc::constants::BootstrapError;
use std::ffi::OsStr;
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

pub fn service_exist(service_name: &str) -> bool {
    let manager_access = ServiceManagerAccess::CONNECT;
    if let Ok(service_manager) = ServiceManager::local_computer(None::<&str>, manager_access) {
        if let Ok(service) = service_manager.open_service(service_name, ServiceAccess::QUERY_CONFIG)
        {
            return true;
        }
    }
    false
}

/// TODO error handle using match instead.
/// It works though.
pub fn start_service(service_name: &str) -> Result<bool, BootstrapError> {
    if !service_exist(service_name) {
        return Err(BootstrapError::ServiceMissing(service_name.to_string()));
    }
    let manager_access = ServiceManagerAccess::CONNECT;
    if let Ok(service_manager) = ServiceManager::local_computer(None::<&str>, manager_access) {
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::START | ServiceAccess::STOP;
        if let Ok(service) = service_manager.open_service(service_name, service_access) {
            //TODO don't blindly unwrap here, add some error handling.
            let service_status = service.query_status().unwrap();
            if service_status.current_state != ServiceState::Stopped {
                service.stop().unwrap();
            }
            service.start(&[OsStr::new("Started from Rust!")]).unwrap();
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn delete_service(service_name: &str) -> bool {
    if service_exist(service_name) {
        let manager_access = ServiceManagerAccess::CONNECT;
        if let Ok(service_manager) = ServiceManager::local_computer(None::<&str>, manager_access) {
            let service_access =
                ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
            if let Ok(service) = service_manager.open_service(service_name, service_access) {
                let service_status = service.query_status().unwrap();
                if service_status.current_state != ServiceState::Stopped {
                    service.stop();
                }
                service.delete();
                return true;
            }
        }
    }
    false
}
