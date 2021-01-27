use crate::etc::constants::BootstrapError;
use crate::os::dacl;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use windows_service::{
    service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

pub struct WindowsService {
    pub name: String,
    pub display_name: String,
    pub executable_path: PathBuf,
    pub arguments: Vec<String>,
}

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

pub fn install_service(service: WindowsService) -> Result<bool, BootstrapError> {
    if service_exist(&service.name) {
        return Err(BootstrapError::ServiceInstalled(service.display_name));
    }
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = match ServiceManager::local_computer(None::<&str>, manager_access) {
        Ok(sm) => sm,
        Err(_e) => return Err(BootstrapError::ServiceConnectionFailure),
    };

    let arguments: Vec<OsString> = service
        .arguments
        .into_iter()
        .map(|x| OsString::from(x))
        .rev()
        .collect();
    let service_info = ServiceInfo {
        name: OsString::from(service.name.clone()),
        display_name: OsString::from(service.display_name),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::OnDemand,
        error_control: ServiceErrorControl::Normal,
        executable_path: service.executable_path,
        launch_arguments: arguments,
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };

    if let Err(_) = service_manager.create_service(service_info, ServiceAccess::empty()) {
        return Err(BootstrapError::ServiceInstallFailed);
    }

    // if the previous call to create a manager didnt fail then this shouldnt fail either
    grant_start_access_rights(&service.name)
}

/// Update the permissions so that any user can start the specified service
pub fn grant_start_access_rights(service_name: &str) -> Result<bool, BootstrapError> {
    let custom_manager = match dacl::CustomServiceManager::new() {
        Ok(sm) => sm,
        Err(_e) => return Err(BootstrapError::ServiceConnectionFailure),
    };
    match custom_manager.change_service_dacl(service_name) {
        Ok(_) => {
            log::info!("Successfully updated service");
            Ok(true)
        }
        Err(_) => Ok(false),
    }
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
        Ok(_o) => Ok(true),
        Err(_e) => Err(BootstrapError::ServiceOpenFailure),
    }
}
