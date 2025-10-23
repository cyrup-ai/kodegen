//! macOS helper app creation and C code generation
//!
//! This module provides macOS-specific helper app creation functionality
//! including C code generation for privilege escalation with zero allocation
//! patterns and blazing-fast performance.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Build and sign macOS helper app with optimized creation
pub fn build_and_sign_helper(out_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let helper_dir = out_dir.join("KodegenHelper.app");

    // Create app bundle structure
    let contents_dir = helper_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    fs::create_dir_all(&macos_dir)?;

    // Create helper executable
    let helper_path = macos_dir.join("KodegenHelper");
    create_helper_executable(&helper_path)?;

    // Create Info.plist
    let info_plist_path = contents_dir.join("Info.plist");
    create_info_plist(&info_plist_path)?;

    // Sign the helper app
    crate::sign_helper::sign_helper_app(&helper_dir)?;

    // Create ZIP for embedding
    let zip_path = crate::package_helper::create_helper_zip(&helper_dir, out_dir)?;

    Ok(zip_path)
}

/// Create helper executable with embedded C code
pub fn create_helper_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create a minimal helper executable using cc
    let helper_code = r#"
#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <signal.h>
#include <errno.h>
#ifdef __APPLE__
#include <libproc.h>
#include <sys/proc_info.h>
#endif

#define SCRIPT_MAX_SIZE 1048576  // 1MB max script size
#define TIMEOUT_SECONDS 300      // 5 minute timeout

#ifdef __APPLE__
#define PROC_PIDPATHINFO_MAXSIZE 4096
#endif

// Global variable to store child PID for signal handler
// volatile because accessed from signal handler
// static to limit scope to this file
static volatile pid_t child_pid_for_timeout = 0;

// Signal handler for timeout
void timeout_handler(int sig) {
    const char msg[] = "Helper: Script execution timed out after 300 seconds\n";
    write(STDERR_FILENO, msg, sizeof(msg) - 1);
    
    // Kill child process if it exists
    if (child_pid_for_timeout > 0) {
        kill(child_pid_for_timeout, SIGKILL);
    }
    
    _exit(124);
}

int main(int argc, char *argv[]) {
    // Verify parent process is kodegend daemon
    pid_t parent_pid = getppid();
    char parent_path[1024];
    snprintf(parent_path, sizeof(parent_path), "/proc/%d/exe", parent_pid);
    
#ifdef __APPLE__
    // Validate parent process with exact path matching
    char parent_name[PROC_PIDPATHINFO_MAXSIZE];
    int path_ret = proc_pidpath(parent_pid, parent_name, sizeof(parent_name));
    
    // CRITICAL: Fail if we cannot get parent path (fail-secure)
    if (path_ret <= 0) {
        fprintf(stderr, "Helper: Failed to get parent process path (errno=%d)\n", errno);
        exit(1);
    }
    
    // Validate EXACT path - no substring matching
    const char *expected_path = "/usr/local/bin/kodegend";
    if (strcmp(parent_name, expected_path) != 0) {
        fprintf(stderr, "Helper: Unauthorized parent process: %s\n", parent_name);
        fprintf(stderr, "Helper: Expected: %s\n", expected_path);
        exit(1);
    }
    
    // Prevent PID reuse attack: Store parent start time
    struct proc_bsdinfo parent_info;
    int info_ret = proc_pidinfo(parent_pid, PROC_PIDTBSDINFO, 0, 
                                 &parent_info, sizeof(parent_info));
    if (info_ret <= 0) {
        fprintf(stderr, "Helper: Failed to get parent process info (errno=%d)\n", errno);
        exit(1);
    }
    
    time_t parent_start_time = parent_info.pbi_start_tvsec;
    
    // Validation passed - store start time for later verification
    // (will check again before executing script)
#endif

    if (argc < 2) {
        fprintf(stderr, "Usage: %s <script_content>\n", argv[0]);
        exit(1);
    }

    const char* script_content = argv[1];
    size_t script_len = strlen(script_content);
    
    if (script_len > SCRIPT_MAX_SIZE) {
        fprintf(stderr, "Helper: Script too large (%zu bytes, max %d)\n", 
                script_len, SCRIPT_MAX_SIZE);
        exit(1);
    }

    // Set up timeout handler
    signal(SIGALRM, timeout_handler);
    alarm(TIMEOUT_SECONDS);

    // Create temporary script file
    char temp_path[] = "/tmp/kodegend_helper_XXXXXX";
    int temp_fd = mkstemp(temp_path);
    if (temp_fd == -1) {
        perror("Helper: Failed to create temporary file");
        exit(1);
    }

    // Write script content
    ssize_t written = write(temp_fd, script_content, script_len);
    if (written != (ssize_t)script_len) {
        perror("Helper: Failed to write script content");
        close(temp_fd);
        unlink(temp_path);
        exit(1);
    }
    close(temp_fd);

#ifdef __APPLE__
    // Verify parent process hasn't changed (PID reuse attack prevention)
    struct proc_bsdinfo current_parent_info;
    int verify_ret = proc_pidinfo(parent_pid, PROC_PIDTBSDINFO, 0,
                                   &current_parent_info, sizeof(current_parent_info));
    
    if (verify_ret <= 0 || current_parent_info.pbi_start_tvsec != parent_start_time) {
        fprintf(stderr, "Helper: Parent process changed during execution\n");
        unlink(temp_path);
        exit(1);
    }
#endif

    // Execute script with elevated privileges (sh reads file, no +x needed)
    pid_t child_pid = fork();
    
    // Store child PID for signal handler
    if (child_pid > 0) {
        child_pid_for_timeout = child_pid;
    }
    
    if (child_pid == 0) {
        // Child process - execute the script
        execl("/bin/sh", "sh", temp_path, NULL);
        perror("Helper: Failed to execute script");
        exit(1);
    } else if (child_pid > 0) {
        // Parent process - wait for completion
        int status;
        if (waitpid(child_pid, &status, 0) == -1) {
            perror("Helper: Failed to wait for child process");
            unlink(temp_path);
            exit(1);
        }

        // Clean up temporary file
        unlink(temp_path);

        // Cancel timeout
        alarm(0);

        // Return child exit status
        if (WIFEXITED(status)) {
            exit(WEXITSTATUS(status));
        } else if (WIFSIGNALED(status)) {
            fprintf(stderr, "Helper: Script terminated by signal %d\n", WTERMSIG(status));
            exit(128 + WTERMSIG(status));
        } else {
            fprintf(stderr, "Helper: Script terminated abnormally\n");
            exit(1);
        }
    } else {
        perror("Helper: Failed to fork");
        unlink(temp_path);
        exit(1);
    }

    return 0;
}
"#;

    // Write C source to temporary file
    // Create secure temporary directory with 0700 permissions
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create secure temp directory: {e}"))?;
    
    let c_source_path = temp_dir.path().join("helper.c");
    fs::write(&c_source_path, helper_code)?;

    // Compile with cc
    let output = Command::new("cc")
        .args([
            "-o",
            path.to_str().ok_or("Invalid path")?,
            c_source_path.to_str().ok_or("Invalid temp path")?,
            "-framework",
            "CoreFoundation",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to compile helper: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Automatic cleanup via TempDir::drop()

    Ok(())
}

/// Create Info.plist for macOS app bundle
pub fn create_info_plist(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>KodegenHelper</string>
    <key>CFBundleIdentifier</key>
    <string>ai.kodegen.kodegend.helper</string>
    <key>CFBundleName</key>
    <string>Kodegen Helper</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>LSUIElement</key>
    <true/>
    <key>SMPrivilegedExecutables</key>
    <dict>
        <key>ai.kodegen.kodegend.helper</key>
        <string>identifier "ai.kodegen.kodegend.helper" and anchor apple generic</string>
    </dict>
    <key>SMAuthorizedClients</key>
    <array>
        <string>identifier "ai.kodegen.kodegend" and anchor apple generic</string>
    </array>
</dict>
</plist>"#;

    fs::write(path, plist_content)?;
    Ok(())
}

/// Validate helper app structure
pub fn validate_helper_structure(helper_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Check required directories exist
    let contents_dir = helper_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");

    if !contents_dir.exists() {
        return Err("Contents directory missing".into());
    }

    if !macos_dir.exists() {
        return Err("MacOS directory missing".into());
    }

    // Check required files exist
    let executable_path = macos_dir.join("KodegenHelper");
    let plist_path = contents_dir.join("Info.plist");

    if !executable_path.exists() {
        return Err("Helper executable missing".into());
    }

    if !plist_path.exists() {
        return Err("Info.plist missing".into());
    }

    // Check executable permissions
    let metadata = fs::metadata(&executable_path)?;
    let permissions = metadata.permissions();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = permissions.mode();
        if (mode & 0o111) == 0 {
            return Err("Helper executable not executable".into());
        }
    }

    Ok(())
}

/// Check if helper app is properly signed
#[must_use] 
pub fn is_helper_signed(helper_dir: &Path) -> bool {
    let executable_path = helper_dir.join("Contents/MacOS/KodegenHelper");

    if !executable_path.exists() {
        return false;
    }

    // Check code signature using codesign
    let output = Command::new("codesign")
        .args(["-v", executable_path.to_str().unwrap_or("")])
        .output();

    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}
