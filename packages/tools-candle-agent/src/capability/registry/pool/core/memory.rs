use sysinfo::System;

/// Query total system memory in MB
pub fn query_system_memory_mb() -> usize {
    let mut sys = System::new_all();
    sys.refresh_memory();
    (sys.total_memory() / 1024 / 1024) as usize
}
