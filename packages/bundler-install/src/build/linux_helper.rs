//! Linux helper executable builder for kodegen
//!
//! This module creates a Linux helper executable that can be embedded
//! into the main binary for cross-platform deployment.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build and sign Linux helper executable
pub fn build_and_sign_helper() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let exe_path = out_dir.join("KodegenHelper");

    // Build Linux helper executable
    create_helper_executable(&exe_path)?;

    // Sign executable (optional but recommended)
    if let Err(e) = sign_executable(&exe_path) {
        eprintln!("Warning: Failed to sign Linux helper executable: {}", e);
    }

    // Generate integrity hash for embedding
    generate_integrity_hash(&exe_path)?;

    println!("cargo:rustc-env=LINUX_HELPER_PATH={}", exe_path.display());

    Ok(())
}

/// Create the Linux helper executable with production-quality C code
fn create_helper_executable(exe_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Create functional Linux helper equivalent to Windows helper but for Linux
    let helper_code = r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <signal.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>

#ifdef __linux__
#include <sys/prctl.h>
#include <linux/limits.h>
#endif

#define SCRIPT_MAX_SIZE 1048576  // 1MB max script size
#define TIMEOUT_SECONDS 300      // 5 minute timeout

// Signal handler for timeout
void timeout_handler(int sig) {
    fprintf(stderr, "Helper: Script execution timed out after %d seconds\n", TIMEOUT_SECONDS);
    exit(124); // Standard timeout exit code
}

// Security function to get and validate parent process (Linux equivalent of Windows version)
int validate_parent_process() {
    pid_t parent_pid = getppid();
    char proc_path[256];
    char parent_name[256];
    ssize_t len;

    
    // Read parent process name from /proc
    snprintf(proc_path, sizeof(proc_path), "/proc/%d/comm", parent_pid);
    
    FILE* comm_file = fopen(proc_path, "r");
    if (!comm_file) {
        fprintf(stderr, "Helper: Failed to open parent process comm file\n");
        return 0;
    }
    
    if (!fgets(parent_name, sizeof(parent_name), comm_file)) {
        fprintf(stderr, "Helper: Failed to read parent process name\n");
        fclose(comm_file);
        return 0;
    }
    fclose(comm_file);
    
    // Remove newline if present
    len = strlen(parent_name);
    if (len > 0 && parent_name[len-1] == '\n') {
        parent_name[len-1] = '\0';
    }
    
    // Check if parent is authorized kodegend/kodegen process
    if (!strstr(parent_name, "kodegend") && !strstr(parent_name, "kodegen")) {
        fprintf(stderr, "Helper: Unauthorized parent process: %s\n", parent_name);
        return 0;
    }
    
    return 1;
}

int main(int argc, char *argv[]) {
    // Critical security: Validate parent process
    if (!validate_parent_process()) {
        exit(1);
    }

    if (argc < 2) {
        fprintf(stderr, "Usage: %s <script_content>\n", argv[0]);
        exit(1);
    }

    const char* script_content = argv[1];
    if (!script_content) {
        fprintf(stderr, "Helper: Script content is NULL\n");
        exit(1);
    }
    
    size_t script_len = strlen(script_content);
    
    if (script_len > SCRIPT_MAX_SIZE) {
        fprintf(stderr, "Helper: Script too large (%zu bytes, max %d)\n", 
                script_len, SCRIPT_MAX_SIZE);
        exit(1);
    }

    // Set up timeout signal handler
    signal(SIGALRM, timeout_handler);
    alarm(TIMEOUT_SECONDS);

    // Create temporary script file
    char temp_path[] = "/tmp/kodegend_helper_XXXXXX";
    int temp_fd = mkstemp(temp_path);
    if (temp_fd == -1) {
        perror("Helper: Failed to create temporary file");
        exit(1);
    }

    // Write script content with comprehensive error handling
    ssize_t written = write(temp_fd, script_content, script_len);
    if (written != (ssize_t)script_len) {
        perror("Helper: Failed to write script content");
        close(temp_fd);
        unlink(temp_path);
        exit(1);
    }
    
    if (close(temp_fd) != 0) {
        perror("Helper: Failed to close temporary file");
        unlink(temp_path);
        exit(1);
    }

    // Make script executable
    if (chmod(temp_path, 0755) != 0) {
        perror("Helper: Failed to make script executable");
        unlink(temp_path);
        exit(1);
    }

    // Execute script with elevated privileges
    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Child process - execute the script
        execl("/bin/sh", "sh", temp_path, NULL);
        perror("Helper: Failed to execute script");
        exit(1);
    } else if (child_pid > 0) {
        // Parent process - wait for completion
        int status;
        pid_t wait_result = waitpid(child_pid, &status, 0);
        if (wait_result == -1) {
            perror("Helper: Failed to wait for child process");
            kill(child_pid, SIGTERM);  // Try to clean up child
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

    // Write the C source code
    let c_path = exe_path.with_extension("c");
    std::fs::write(&c_path, helper_code)?;

    // Compile with gcc (standard Linux compiler)
    let output = Command::new("gcc")
        .args(&[
            "-std=c99",
            "-D_GNU_SOURCE",
            "-o",
            &exe_path.to_string_lossy(),
            &c_path.to_string_lossy(),
        ])
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                return Err(format!(
                    "Failed to compile Linux helper with GCC: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }
        Err(_) => {
            return Err("GCC compiler not found - required for Linux helper compilation".into());
        }
    }

    // Clean up temporary C file
    let _ = std::fs::remove_file(c_path);

    // Verify the executable was created - FAIL BUILD if not
    if !exe_path.exists() {
        return Err("Failed to create Linux helper executable - compilation failed".into());
    }

    Ok(())
}

/// Sign the Linux executable using GPG (standalone build-time implementation)
fn sign_executable(exe_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::env;

    // Find GPG binary (standalone implementation)
    let gpg = find_gpg_binary()?;

    // Get GPG key ID from environment
    let key_id = env::var("GPG_KEY_ID").ok();

    // Build GPG signing arguments
    let mut args = vec!["--detach-sign".to_string(), "--armor".to_string()];

    if let Some(key) = key_id {
        args.push("--local-user".to_string());
        args.push(key);
    }

    let sig_path = exe_path.with_extension("sig");
    args.push("--output".to_string());
    args.push(sig_path.to_string_lossy().to_string());
    args.push(exe_path.to_string_lossy().to_string());

    // Execute GPG signing
    let output = Command::new(&gpg).args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("GPG signing failed: {}", stderr).into());
    }

    println!(
        "Successfully signed Linux helper executable: {}",
        sig_path.display()
    );
    Ok(())
}

/// Find GPG binary (standalone build-time version)
fn find_gpg_binary() -> Result<String, Box<dyn std::error::Error>> {
    use which::which;

    // Try gpg2 first, then gpg
    if let Ok(gpg2) = which("gpg2") {
        return Ok(gpg2.to_string_lossy().to_string());
    }

    if let Ok(gpg) = which("gpg") {
        return Ok(gpg.to_string_lossy().to_string());
    }

    Err("GPG not found. Please install gpg or gpg2".into())
}

/// Generate integrity hash for the executable
fn generate_integrity_hash(exe_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};

    let exe_data = std::fs::read(exe_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&exe_data);
    let hash = hasher.finalize();

    let hash_hex = hex::encode(hash);
    let hash_path = exe_path.with_extension("sha256");

    std::fs::write(&hash_path, &hash_hex)?;

    println!("cargo:rustc-env=LINUX_HELPER_HASH={}", hash_hex);

    Ok(())
}
