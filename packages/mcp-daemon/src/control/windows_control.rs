//! Windows daemon control using Service Control Manager (SCM) API

use anyhow::{Context, Result};
use std::mem;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx,
    StartServiceW, SC_HANDLE, SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO,
    SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START,
    SERVICE_STATUS, SERVICE_STATUS_PROCESS, SERVICE_STOP,
};

const SERVICE_NAME: &str = "kodegend";

/// RAII wrapper for SC_HANDLE (Service Control Manager handle)
struct ScManagerHandle(SC_HANDLE);

impl ScManagerHandle {
    fn new() -> Result<Self> {
        let handle = unsafe {
            OpenSCManagerW(
                PCWSTR::null(),
                PCWSTR::null(),
                SC_MANAGER_CONNECT.0,
            )
        };

        if handle.is_invalid() {
            anyhow::bail!("Failed to open Service Control Manager");
        }

        Ok(ScManagerHandle(handle))
    }

    fn handle(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for ScManagerHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseServiceHandle(self.0);
            }
        }
    }
}

/// RAII wrapper for SC_HANDLE (Service handle)
struct ServiceHandle(SC_HANDLE);

impl ServiceHandle {
    fn handle(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseServiceHandle(self.0);
            }
        }
    }
}

/// Open a service with the specified access rights
fn open_service(sc_manager: &ScManagerHandle, access: u32) -> Result<ServiceHandle> {
    let service_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(Some(0)).collect();

    let handle = unsafe {
        OpenServiceW(
            sc_manager.handle(),
            PCWSTR(service_name.as_ptr()),
            access,
        )
    };

    if handle.is_invalid() {
        anyhow::bail!("Failed to open service: {}", SERVICE_NAME);
    }

    Ok(ServiceHandle(handle))
}

/// Check if daemon is running via QueryServiceStatusEx
///
/// Returns: Ok(true) if service is running, Ok(false) if stopped
pub fn check_status() -> Result<bool> {
    let sc_manager = ScManagerHandle::new()
        .context("Failed to open Service Control Manager for status check")?;

    let service = open_service(&sc_manager, SERVICE_QUERY_STATUS.0)
        .context("Failed to open service for status check")?;

    let mut status: SERVICE_STATUS_PROCESS = unsafe { mem::zeroed() };
    let mut bytes_needed: u32 = 0;

    let result = unsafe {
        QueryServiceStatusEx(
            service.handle(),
            SC_STATUS_PROCESS_INFO,
            Some(&mut status as *mut _ as *mut u8),
            mem::size_of::<SERVICE_STATUS_PROCESS>() as u32,
            &mut bytes_needed,
        )
    };

    if result.is_err() {
        anyhow::bail!("Failed to query service status");
    }

    // SERVICE_RUNNING = 4, SERVICE_STOPPED = 1
    Ok(status.dwCurrentState == SERVICE_RUNNING.0)
}

/// Start daemon via StartServiceW
pub fn start_daemon() -> Result<()> {
    let sc_manager = ScManagerHandle::new()
        .context("Failed to open Service Control Manager for start")?;

    let service = open_service(&sc_manager, SERVICE_START.0)
        .context("Failed to open service for start")?;

    let result = unsafe {
        StartServiceW(service.handle(), None)
    };

    if result.is_err() {
        anyhow::bail!("Failed to start service");
    }

    Ok(())
}

/// Stop daemon via ControlService
pub fn stop_daemon() -> Result<()> {
    let sc_manager = ScManagerHandle::new()
        .context("Failed to open Service Control Manager for stop")?;

    let service = open_service(&sc_manager, SERVICE_STOP.0)
        .context("Failed to open service for stop")?;

    let mut status: SERVICE_STATUS = unsafe { mem::zeroed() };

    let result = unsafe {
        ControlService(service.handle(), SERVICE_CONTROL_STOP, &mut status)
    };

    if result.is_err() {
        anyhow::bail!("Failed to stop service");
    }

    Ok(())
}

/// Restart daemon (Windows doesn't have native restart - stop + start)
pub fn restart_daemon() -> Result<()> {
    // Stop the service
    stop_daemon()?;

    // Wait for service to fully stop
    std::thread::sleep(Duration::from_secs(1));

    // Start the service
    start_daemon()?;

    Ok(())
}
