//! Windows service executable builder for kodegen
//!
//! This module creates a Windows service executable that can be embedded
//! into the main binary for cross-platform deployment.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Build and optionally sign the Windows helper executable
pub fn build_and_sign_helper() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let exe_path = out_dir.join("KodegenHelper.exe");

    // Build Windows service executable
    create_service_executable(&exe_path)?;

    // Sign executable (optional but recommended)
    if let Err(e) = sign_executable(&exe_path) {
        eprintln!("Warning: Failed to sign executable: {}", e);
    }

    // Generate integrity hash for embedding
    generate_integrity_hash(&exe_path)?;

    println!(
        "cargo:rustc-env=WINDOWS_HELPER_EXE_PATH={}",
        exe_path.display()
    );

    Ok(())
}

/// Create the Windows service executable
fn create_service_executable(exe_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Create functional Windows helper equivalent to macOS helper
    create_functional_executable(exe_path)?;
    Ok(())
}

/// Create functional Windows helper executable using C code (like macOS helper)
fn create_functional_executable(exe_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Use the same sophisticated helper pattern as macOS
    let helper_code = r#"
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <process.h>
#include <io.h>
#include <psapi.h>
#include <tlhelp32.h>
#include <shlwapi.h>

#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "psapi.lib")
#pragma comment(lib, "shlwapi.lib")

#define SCRIPT_MAX_SIZE 1048576  // 1MB max script size
#define TIMEOUT_SECONDS 300      // 5 minute timeout

// Timeout handler using Windows APIs
VOID CALLBACK TimeoutCallback(PVOID lpParam, BOOLEAN TimerOrWaitFired) {
    fprintf(stderr, "Helper: Script execution timed out after %d seconds\n", TIMEOUT_SECONDS);
    ExitProcess(124); // Standard timeout exit code
}

// Security function to get and validate parent process
BOOL ValidateParentProcess() {
    DWORD current_pid = GetCurrentProcessId();
    DWORD parent_pid = 0;
    
    // Get parent PID using CreateToolhelp32Snapshot
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snapshot == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "Helper: Failed to create process snapshot\n");
        return FALSE;
    }
    
    PROCESSENTRY32 pe32;
    pe32.dwSize = sizeof(PROCESSENTRY32);
    
    if (!Process32First(snapshot, &pe32)) {
        fprintf(stderr, "Helper: Failed to enumerate processes\n");
        CloseHandle(snapshot);
        return FALSE;
    }
    
    // Find our process and get parent PID
    do {
        if (pe32.th32ProcessID == current_pid) {
            parent_pid = pe32.th32ParentProcessID;
            break;
        }
    } while (Process32Next(snapshot, &pe32));
    
    CloseHandle(snapshot);
    
    if (parent_pid == 0) {
        fprintf(stderr, "Helper: Could not find parent process\n");
        return FALSE;
    }
    
    // Validate parent process
    HANDLE parent_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, parent_pid);
    if (!parent_handle) {
        fprintf(stderr, "Helper: Failed to open parent process (PID: %lu)\n", parent_pid);
        return FALSE;
    }
    
    char parent_name[MAX_PATH];
    DWORD name_size = sizeof(parent_name);
    if (!GetModuleBaseNameA(parent_handle, NULL, parent_name, name_size)) {
        fprintf(stderr, "Helper: Failed to get parent process name\n");
        CloseHandle(parent_handle);
        return FALSE;
    }
    
    CloseHandle(parent_handle);
    
    // Check if parent is authorized kodegend/kodegen process
    if (!strstr(parent_name, "kodegend") && !strstr(parent_name, "kodegen")) {
        fprintf(stderr, "Helper: Unauthorized parent process: %s\n", parent_name);
        return FALSE;
    }
    
    return TRUE;
}

int main(int argc, char *argv[]) {
    // Critical security: Validate parent process using proper enumeration
    if (!ValidateParentProcess()) {
        ExitProcess(1);
    }

    if (argc < 2) {
        fprintf(stderr, "Usage: %s <script_content>\n", argv[0]);
        ExitProcess(1);
    }

    const char* script_content = argv[1];
    if (!script_content) {
        fprintf(stderr, "Helper: Script content is NULL\n");
        ExitProcess(1);
    }
    
    size_t script_len = strlen(script_content);
    
    if (script_len > SCRIPT_MAX_SIZE) {
        fprintf(stderr, "Helper: Script too large (%zu bytes, max %d)\n", 
                script_len, SCRIPT_MAX_SIZE);
        ExitProcess(1);
    }

    // Set up timeout using Windows timer with proper error handling
    HANDLE timer_queue = CreateTimerQueue();
    if (!timer_queue) {
        fprintf(stderr, "Helper: Failed to create timer queue\n");
        ExitProcess(1);
    }
    
    HANDLE timer = NULL;
    if (!CreateTimerQueueTimer(&timer, timer_queue, TimeoutCallback, NULL, 
                              TIMEOUT_SECONDS * 1000, 0, 0)) {
        fprintf(stderr, "Helper: Failed to create timer\n");
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }

    // Create temporary script file with secure path operations
    char temp_dir[MAX_PATH];
    DWORD temp_dir_len = GetTempPathA(sizeof(temp_dir), temp_dir);
    if (temp_dir_len == 0 || temp_dir_len >= sizeof(temp_dir)) {
        fprintf(stderr, "Helper: Failed to get temp directory\n");
        if (timer) DeleteTimerQueueTimer(timer_queue, timer, NULL);
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }
    
    char temp_path[MAX_PATH];
    // Use secure PathCombineA instead of strcat to prevent buffer overflow
    if (!PathCombineA(temp_path, temp_dir, "kodegend_helper_script.bat")) {
        fprintf(stderr, "Helper: Failed to create temp file path\n");
        if (timer) DeleteTimerQueueTimer(timer_queue, timer, NULL);
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }
    
    // Write script content with comprehensive error handling
    FILE* temp_file = fopen(temp_path, "w");
    if (!temp_file) {
        fprintf(stderr, "Helper: Failed to create temporary file: %s\n", strerror(errno));
        if (timer) DeleteTimerQueueTimer(timer_queue, timer, NULL);
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }
    
    if (fwrite(script_content, 1, script_len, temp_file) != script_len) {
        fprintf(stderr, "Helper: Failed to write script content: %s\n", strerror(errno));
        fclose(temp_file);
        DeleteFileA(temp_path);
        if (timer) DeleteTimerQueueTimer(timer_queue, timer, NULL);
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }
    
    if (fclose(temp_file) != 0) {
        fprintf(stderr, "Helper: Failed to close temporary file: %s\n", strerror(errno));
        DeleteFileA(temp_path);
        if (timer) DeleteTimerQueueTimer(timer_queue, timer, NULL);
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }

    // Execute script with elevated privileges and comprehensive error handling
    STARTUPINFOA si = {0};
    PROCESS_INFORMATION pi = {0};
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESHOWWINDOW;
    si.wShowWindow = SW_HIDE; // Hide console window
    
    char command[MAX_PATH * 2];
    int cmd_len = snprintf(command, sizeof(command), "cmd.exe /C \"%s\"", temp_path);
    if (cmd_len < 0 || cmd_len >= sizeof(command)) {
        fprintf(stderr, "Helper: Command line too long\n");
        DeleteFileA(temp_path);
        if (timer) DeleteTimerQueueTimer(timer_queue, timer, NULL);
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }
    
    if (!CreateProcessA(NULL, command, NULL, NULL, FALSE, CREATE_NO_WINDOW, 
                       NULL, NULL, &si, &pi)) {
        DWORD error = GetLastError();
        fprintf(stderr, "Helper: Failed to execute script (error %lu)\n", error);
        DeleteFileA(temp_path);
        if (timer) DeleteTimerQueueTimer(timer_queue, timer, NULL);
        DeleteTimerQueue(timer_queue);
        ExitProcess(1);
    }

    // Wait for completion with proper handle management
    DWORD wait_result = WaitForSingleObject(pi.hProcess, INFINITE);
    if (wait_result != WAIT_OBJECT_0) {
        fprintf(stderr, "Helper: Wait failed with result %lu\n", wait_result);
        TerminateProcess(pi.hProcess, 1);
    }
    
    DWORD exit_code = 1;
    if (!GetExitCodeProcess(pi.hProcess, &exit_code)) {
        fprintf(stderr, "Helper: Failed to get process exit code\n");
        exit_code = 1;
    }
    
    // Cleanup with proper error handling
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    DeleteFileA(temp_path);
    
    // Cancel timeout with proper cleanup
    if (timer) {
        DeleteTimerQueueTimer(timer_queue, timer, NULL);
    }
    DeleteTimerQueue(timer_queue);
    
    ExitProcess(exit_code);
    return 0;
}
"#;

    // Write the C source code
    let c_path = exe_path.with_extension("c");
    std::fs::write(&c_path, helper_code)?;

    // Try to compile with available compiler - include required libraries
    if let Ok(output) = Command::new("cl")
        .args(&[
            "/Fe:",
            &exe_path.to_string_lossy(),
            &c_path.to_string_lossy(),
            "kernel32.lib",
            "psapi.lib",
            "shlwapi.lib",
        ])
        .output()
    {
        if !output.status.success() {
            compile_with_mingw_c(&c_path, exe_path)?;
        }
    } else {
        compile_with_mingw_c(&c_path, exe_path)?;
    }

    // Clean up temporary C file
    let _ = std::fs::remove_file(c_path);

    // Verify the executable was created - FAIL BUILD if not
    if !exe_path.exists() {
        return Err(
            "Failed to create Windows helper executable - no suitable compiler found".into(),
        );
    }

    Ok(())
}

/// Compile with MinGW for C code
fn compile_with_mingw_c(
    c_path: &PathBuf,
    exe_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("gcc")
        .args(&[
            "-std=c99",
            "-o",
            &exe_path.to_string_lossy(),
            &c_path.to_string_lossy(),
            "-lkernel32",
            "-lpsapi",
            "-lshlwapi",
        ])
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                return Err(format!(
                    "Failed to compile Windows helper with GCC: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }
        Err(_) => {
            return Err("No suitable C compiler found (tried cl.exe and gcc)".into());
        }
    }

    Ok(())
}

/// Sign the Windows executable (optional)
fn sign_executable(exe_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Try to sign with signtool if available
    if let Ok(output) = Command::new("signtool")
        .args(&[
            "sign",
            "/a",
            "/fd",
            "SHA256",
            "/t",
            "http://timestamp.digicert.com",
            &exe_path.to_string_lossy(),
        ])
        .output()
    {
        if !output.status.success() {
            eprintln!(
                "Warning: Code signing failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        } else {
            println!("Successfully signed Windows helper executable");
        }
    } else {
        eprintln!("Warning: signtool not found, executable will be unsigned");
    }

    Ok(())
}

/// Generate integrity hash for the executable
fn generate_integrity_hash(exe_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};

    let exe_data = std::fs::read(exe_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&exe_data);
    let hash = hasher.finalize();

    let hash_hex = hex::encode(hash);
    let hash_path = exe_path.with_extension("exe.sha256");

    std::fs::write(&hash_path, &hash_hex)?;

    println!("cargo:rustc-env=WINDOWS_HELPER_EXE_HASH={}", hash_hex);

    Ok(())
}
