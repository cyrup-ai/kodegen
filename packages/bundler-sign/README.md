# kodegen_sign

**Automated code signing and certificate provisioning for Kodegen daemon**

## Overview

`kodegen_sign` is a comprehensive tool for managing code signing workflows across macOS, Linux, and Windows platforms. Its primary focus is **macOS**, where it provides automated certificate provisioning through Apple's App Store Connect API, builds and signs privileged helper applications, and manages deployment to GitHub releases.

The package serves two main purposes:

1. **Certificate Provisioning**: Automates the process of obtaining Developer ID Application certificates from Apple using App Store Connect API credentials
2. **Helper App Management**: Builds, signs, packages, and deploys the macOS privileged helper application (`KodegenHelper.app`) that enables the Kodegen daemon to execute administrative tasks

## Quick Start

### Interactive Setup (Recommended)

```bash
cargo run --package kodegen_sign -- --interactive
```

This will guide you through:
1. Checking for existing certificates
2. Providing App Store Connect API credentials
3. Generating and requesting a Developer ID certificate
4. Installing it to your keychain

### Build Helper App

```bash
cargo run --package kodegen_sign -- --build-helper
```

### Build and Upload to GitHub

```bash
export GITHUB_TOKEN="your_token"
cargo run --package kodegen_sign -- --build-helper --upload
```

## Architecture

### Core Modules

- **`lib.rs`**: Library interface with platform-conditional exports
- **`main.rs`**: CLI application with four operational modes
- **`config.rs`**: Configuration structures for all platforms
- **`error.rs`**: Custom error types using `thiserror`

### Platform-Specific Modules

- **`apple_api.rs`**: App Store Connect API client for certificate provisioning
- **`macos.rs`**: macOS certificate provisioning and keychain management
- **`build_helper.rs`**: macOS helper app creation and C code compilation
- **`sign_helper.rs`**: Code signing operations with codesign
- **`package_helper.rs`**: ZIP packaging and integrity hashing
- **`linux.rs`**: Linux setup guidance (GPG-based)
- **`windows.rs`**: Windows setup guidance (Authenticode-based)

## CLI Modes

### 1. Show Configuration

```bash
cargo run --package kodegen_sign -- --show
```

Displays:
- Developer ID certificates in keychain
- Configuration file location (`~/.config/kodegen/signing.toml`)

### 2. Interactive Setup

```bash
cargo run --package kodegen_sign -- --interactive
```

Guided setup with prompts for:
- App Store Connect Issuer ID
- API Key ID
- Path to .p8 private key file
- Email address

### 3. Build Helper Mode

```bash
cargo run --package kodegen_sign -- --build-helper [OPTIONS]
```

Options:
- `--upload`: Upload to GitHub releases
- `--github-token <TOKEN>`: GitHub API token (or use `GITHUB_TOKEN` env var)
- `--output-dir <DIR>`: Output directory (default: `target/helper`)

### 4. Config File Mode

```bash
cargo run --package kodegen_sign -- --config signing.toml
```

Example `signing.toml`:
```toml
platform = "macos"
dry_run = false
verbose = true

issuer_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
key_id = "XXXXXXXXXX"
private_key_path = "~/.keys/AuthKey_XXXXXXXXXX.p8"
certificate_type = "developer_id"
common_name = "Kodegen Helper"
keychain = "login.keychain-db"
```

## Apple API Integration

### App Store Connect API Client

The `apple_api.rs` module implements JWT-based authentication with Apple's certificate provisioning API.

#### Authentication Flow

1. **Load .p8 Private Key**: ECDSA ES256 private key from App Store Connect
2. **Generate JWT Token**:
   - Algorithm: ES256
   - Header: `kid` (Key ID), `alg: "ES256"`
   - Claims: `iss` (Issuer ID), `aud: "appstoreconnect-v1"`, `iat`, `exp` (20 minutes)
3. **Sign JWT**: Sign with private key using `jsonwebtoken` crate
4. **API Request**: Include JWT as Bearer token in Authorization header

#### Certificate Request API

**Endpoint**: `https://api.appstoreconnect.apple.com/v1/certificates`

**Request**:
```json
{
  "data": {
    "type": "certificates",
    "attributes": {
      "certificateType": "DEVELOPER_ID_APPLICATION",
      "csrContent": "<PEM-encoded CSR>"
    }
  }
}
```

**Response**:
```json
{
  "data": {
    "attributes": {
      "certificateContent": "<base64-encoded DER certificate>"
    }
  }
}
```

#### CSR Generation

Uses `rcgen` crate to generate:
- RSA key pair
- Certificate Signing Request with:
  - CommonName: "Kodegen Helper" (or configured name)
  - CountryName: "US"

Returns: (CSR PEM, Private Key PEM)

## Certificate Provisioning Workflow

### Prerequisites

1. **Apple Developer Account** with Admin or Developer role
2. **Create App Store Connect API Key**:
   - Navigate to [App Store Connect](https://appstoreconnect.apple.com/access/api)
   - Go to: Users and Access → Keys → App Store Connect API
   - Click "+" to create new key
   - Name: "Kodegen Signing"
   - Access: **Developer** role
   - Download `.p8` file (**one-time download only**)
   - Note the **Key ID** (10 alphanumeric characters)
   - Note the **Issuer ID** (UUID format)

### Automated Provisioning Process

#### Step 1: API Authentication
- Read `.p8` private key file
- Generate JWT with ES256 signature
- Token lifetime: 20 minutes
- Include Key ID in JWT header

#### Step 2: CSR Generation
- Generate RSA key pair with `rcgen`
- Create Certificate Signing Request
- Fields: CommonName, CountryName

#### Step 3: Request Certificate
- POST CSR to Apple's API
- Receive base64-encoded DER certificate

#### Step 4: Create P12 Bundle
```bash
openssl pkcs12 -export \
  -inkey /tmp/kodegen_key.pem \
  -in /tmp/kodegen_cert.der \
  -out /tmp/kodegen_cert.p12 \
  -passout pass:
```

#### Step 5: Import to Keychain
```bash
security import /tmp/kodegen_cert.p12 \
  -k login.keychain-db \
  -P "" \
  -T /usr/bin/codesign
```

#### Step 6: Save Configuration
Saves credentials to `~/.config/kodegen/signing.toml`

#### Step 7: Cleanup
Removes temporary files: cert.der, key.pem, cert.p12

## Helper App Architecture

### Purpose

`KodegenHelper.app` is a macOS privileged helper that enables the Kodegen daemon to execute administrative tasks without running the entire daemon as root. It follows macOS best practices for privilege separation using the Service Management framework.

### Security Model

**Authorization Requirements**:
- Helper requires `admin` group membership
- Daemon identity must match `SMAuthorizedClients` in Info.plist
- Helper identity must match `SMPrivilegedExecutables` in daemon's Info.plist
- Code signature verification enforced by macOS

**Runtime Security**:
- **Parent Process Validation**: Uses `proc_pidpath` to verify parent is `kodegend`
- **Script Size Limit**: Maximum 1MB (1,048,576 bytes)
- **Execution Timeout**: 5 minutes enforced via `SIGALRM`
- **Temporary File Security**: Uses `mkstemp` for secure random filenames
- **Secure Permissions**: Files remain 0600 (owner-only) throughout execution
- **Automatic Cleanup**: Removes temporary files after execution

### C Source Code Implementation

The helper is implemented in C and compiled with `cc`. Key features:

**Main Function Flow**:
```c
1. Validate parent process name contains "kodegen" (macOS: proc_pidpath)
2. Accept script content as argv[1]
3. Validate script size <= 1MB
4. Set up SIGALRM timeout handler (300 seconds)
5. Create temporary file with mkstemp: /tmp/kodegend_helper_XXXXXX
6. Write script content (0600 permissions preserved for security)
7. Fork child process
8. Child: execl("/bin/sh", "sh", temp_path, NULL)
9. Parent: waitpid for completion
10. Clean up temporary file
11. Return child exit status
```

**Error Handling**:
- Exit code 1: Validation or setup failures
- Exit code 124: Timeout reached
- Exit code 128 + N: Killed by signal N
- Otherwise: Child process exit code

**Compilation**:
```bash
cc -o KodegenHelper kodegend_helper.c -framework CoreFoundation
```

### App Bundle Structure

```
KodegenHelper.app/
├── Contents/
│   ├── Info.plist          # Bundle metadata
│   └── MacOS/
│       └── KodegenHelper   # Compiled C executable
```

### Info.plist Configuration

**Key Settings**:
- Bundle ID: `ai.kodegen.kodegend.helper`
- LSUIElement: `true` (background agent, no dock icon)
- Minimum macOS: 10.15

**Authorization Settings**:
```xml
<key>SMPrivilegedExecutables</key>
<dict>
    <key>ai.kodegen.kodegend.helper</key>
    <string>identifier "ai.kodegen.kodegend.helper" and anchor apple generic</string>
</dict>
<key>SMAuthorizedClients</key>
<array>
    <string>identifier "ai.kodegen.kodegend" and anchor apple generic</string>
</array>
```

### Entitlements

**File**: `helper.entitlements`

```xml
<key>com.apple.security.authorization.groups</key>
<array>
    <string>admin</string>
</array>
<key>com.apple.security.inherit</key>
<true/>
```

**Purpose**:
- Requires admin group membership for execution
- Allows child processes to inherit permissions

## Code Signing Process

### Signing Workflow

The `sign_helper.rs` module implements a multi-step signing process:

#### 1. Certificate Check

```bash
security find-identity -v -p codesigning
```

If no Developer ID certificate found:
- Prints setup guidance
- Falls back to ad-hoc signing (`-`) for development
- Returns signing identity string: "Developer ID Application" or "-" (ad-hoc)
- Does not mutate environment variables (thread-safe)

#### 2. Entitlements Creation

Generates `helper.entitlements` with admin authorization requirements.

#### 3. Sign Executable

```bash
codesign --force --sign <identity> \
         --options runtime \
         --entitlements helper.entitlements \
         Contents/MacOS/KodegenHelper
```

#### 4. Sign App Bundle

```bash
codesign --force --deep --sign <identity> \
         --options runtime \
         KodegenHelper.app
```

#### 5. Verify Signature

```bash
codesign --verify --deep --strict KodegenHelper.app
```

### Signing Identities

**Production**:
- Identity: "Developer ID Application: Your Name (TEAM_ID)"
- Obtained via automated provisioning or manual certificate installation

**Development**:
- Identity: "-" (ad-hoc signing)
- Set via: `export KODEGEN_SIGNING_IDENTITY="-"`
- Warnings instead of errors for signing failures

### Hardened Runtime

All production builds use `--options runtime` flag, enabling:
- Library validation
- Hardened runtime protections
- Required for notarization

## Packaging and Distribution

### ZIP Creation

The `package_helper.rs` module creates compressed packages:

**Features**:
- Compression: Deflated (standard ZLIB)
- Unix Permissions: 0755 preserved
- Recursive directory traversal
- Maintains app bundle structure

**Output Files**:
- `KodegenHelper.app.zip`: Compressed app bundle
- `KodegenHelper.app.zip.sha256`: SHA-256 integrity hash (hex-encoded)
- `app_zip_data.rs`: Generated Rust code with `include_bytes!` macro

### Integrity Verification

**Hash Generation**:
```bash
SHA256(KodegenHelper.app.zip) = <64-character hex string>
```

**Verification Process**:
- Checks for required files: Info.plist, executable
- Validates executable is not empty
- Verifies all files readable without corruption
- Ensures Info.plist is at least 100 bytes

### Build System Integration

**Generated Rust Code**:
```rust
const APP_ZIP_DATA: &[u8] = include_bytes!("/path/to/KodegenHelper.app.zip");
```

**Cargo Environment Variables**:
- `HELPER_ZIP_PATH`: Path to ZIP file
- `HELPER_ZIP_INCLUDE_FILE`: Path to generated Rust file
- `MACOS_HELPER_ZIP_HASH`: SHA-256 hash

### Atomic Operations

The `create_functional_zip` function implements atomic builds:

1. Create temporary working directory
2. Validate output directory is writable
3. Build and sign helper app
4. Validate helper structure
5. Create ZIP in temporary location
6. Verify ZIP integrity
7. **Atomic rename** to final location
8. Cleanup temporary files (success or failure)

**Rollback on Failure**: All temporary files removed if any step fails.

## GitHub Integration

### Upload Process

**Target Repository**: `cyrup-ai/kodegen`

**Workflow**:
1. Build and sign helper app
2. Create ZIP package
3. Calculate SHA-256 hash
4. Detect system architecture (e.g., `aarch64`, `x86_64`)
5. Get latest release via GitHub API
6. Upload as asset: `KodegenHelper.app-macos-{arch}.zip`

**Authentication**:
- GitHub token via `--github-token` flag
- Or `GITHUB_TOKEN` environment variable
- Requires `repo` scope for releases

**Upload Command**:
```bash
cargo run --package kodegen_sign -- \
  --build-helper \
  --upload \
  --github-token "ghp_xxxxxxxxxxxxxxxxxxxx"
```

### Asset Naming Convention

Format: `KodegenHelper.app-macos-{arch}.zip`

Examples:
- `KodegenHelper.app-macos-aarch64.zip` (Apple Silicon)
- `KodegenHelper.app-macos-x86_64.zip` (Intel)

### Download URL Format

```
https://github.com/cyrup-ai/kodegen/releases/download/{tag}/KodegenHelper.app-macos-{arch}.zip
```

## Dependencies

### Core Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `clap` | 4.5 | Command-line argument parsing |
| `anyhow` | 1 | Error handling with context |
| `serde` | 1 | Serialization framework |
| `serde_json` | 1 | JSON parsing |
| `toml` | 0.9 | TOML configuration parsing |
| `thiserror` | 2.0 | Custom error types |

### macOS-Specific Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `rcgen` | 0.14 | CSR and key pair generation |
| `jsonwebtoken` | 9.3 | JWT creation with ES256 |
| `base64` | 0.22 | Base64 encoding/decoding |
| `reqwest` | 0.12 | HTTP client for Apple API |
| `dirs` | 6.0 | Platform-specific paths |
| `shellexpand` | 3.1 | Tilde expansion |

### Build & Packaging Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `cc` | 1.0 | C code compilation |
| `zip` | 2.1 | ZIP archive creation |
| `sha2` | 0.10 | SHA-256 hashing |
| `hex` | 0.4 | Hexadecimal encoding |

### GitHub Integration Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `kodegen_github` | local | GitHub API client |
| `tokio` | 1.48 | Async runtime |
| `bytes` | 1.0 | Byte buffers |
| `octocrab` | 0.42 | GitHub API library |

### System Dependencies (macOS)

- **OpenSSL**: PKCS#12 bundle creation (`openssl` command)
- **Security Framework**: Keychain management (`security` command)
- **Codesign**: Code signature operations (`codesign` command)
- **Xcode Command Line Tools**: C compiler (`cc` command)

## Platform Support

### macOS (Full Support ✅)

**Features**:
- Automated certificate provisioning
- App Store Connect API integration
- Helper app building and compilation
- Code signing with Developer ID
- Hardened runtime support
- ZIP packaging with integrity hashing
- GitHub release uploads

**Requirements**:
- macOS 10.15 or later
- Xcode Command Line Tools
- Apple Developer Account (for certificate provisioning)
- App Store Connect API key

### Linux (Minimal Support ⚠️)

**Current Implementation**:
- GPG setup guidance only
- No automated provisioning
- Configuration parsing supported

**Recommendations**:
```bash
# Generate GPG key
gpg --full-generate-key

# List keys
gpg --list-secret-keys --keyid-format LONG
```

### Windows (Minimal Support ⚠️)

**Current Implementation**:
- Authenticode guidance only
- No automated provisioning
- Configuration parsing supported

**Recommendations**:
```bash
# Import certificate
certutil -user -importpfx code_signing_cert.pfx

# View certificates
certmgr.msc
```

## Security Considerations

### Certificate Security

1. **Private Key Protection**
   - Store `.p8` files securely with restricted permissions: `chmod 600`
   - Never commit to version control
   - Use environment variables in CI/CD
   - Rotate API keys periodically

2. **Keychain Security**
   - Certificates stored in macOS Keychain
   - Access Control List restricts to `/usr/bin/codesign`
   - Requires user authentication on first use

3. **JWT Token Security**
   - Tokens expire after 20 minutes
   - Generated on-demand, not stored
   - Signed with ES256 algorithm

### Helper App Security

1. **Code Signing Verification**
   - Hardened Runtime enabled
   - Signature checked by macOS before execution
   - Tampering detected and prevented

2. **Authorization Model**
   - Requires admin group membership
   - Parent process validation
   - Identity matching via Service Management

3. **Execution Limits**
   - 1MB script size limit prevents abuse
   - 5-minute timeout prevents hangs
   - Automatic cleanup prevents file leaks

4. **Input Validation**
   - Parent process name verification
   - Script size validation
   - Proper error handling for all system calls

### GitHub Upload Security

1. **Token Permissions**
   - Minimum: `repo` scope
   - Recommended: Fine-grained token scoped to repository
   - Never expose tokens in logs or error messages

2. **Asset Verification**
   - SHA-256 hash published with release
   - Users should verify hash before use
   - Download only from official releases

## Troubleshooting

### Certificate Provisioning

**Issue**: `Certificate request failed: Unauthorized`

**Solutions**:
- Verify Issuer ID and Key ID are correct
- Ensure `.p8` file is not corrupted
- Check API key has "Developer" role in App Store Connect
- Confirm API key is not revoked

---

**Issue**: `Failed to import to keychain`

**Solutions**:
```bash
# Unlock keychain
security unlock-keychain login.keychain-db

# List keychains
security list-keychains

# Verify OpenSSL installation
which openssl
openssl version
```

---

**Issue**: `No Developer ID certificate found`

**Solutions**:
- Run interactive setup: `cargo run --package kodegen_sign -- --interactive`
- Or use ad-hoc signing: `export KODEGEN_SIGNING_IDENTITY="-"`

### Helper Building

**Issue**: `Failed to compile helper: cc not found`

**Solutions**:
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Verify installation
cc --version
xcode-select -p
```

---

**Issue**: `Failed to sign executable: code object is not signed at all`

**Solution**: This is a warning in development mode, not an error. The build continues with ad-hoc signing.

---

**Issue**: `Helper validation failed: Helper executable not executable`

**Solutions**:
```bash
# Check permissions
ls -l target/helper/KodegenHelper.app/Contents/MacOS/KodegenHelper

# Fix permissions
chmod +x target/helper/KodegenHelper.app/Contents/MacOS/KodegenHelper
```

### GitHub Upload

**Issue**: `GitHub token required for upload`

**Solutions**:
```bash
# Set environment variable
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"

# Or use CLI flag
cargo run --package kodegen_sign -- --build-helper --upload --github-token "ghp_xxx"
```

---

**Issue**: `Failed to get latest release`

**Solutions**:
- Verify repository exists: `cyrup-ai/kodegen`
- Check token has `repo` scope
- Ensure at least one release exists
- Verify network connectivity

## Development

### Local Development Setup

```bash
# 1. Clone repository
git clone https://github.com/cyrup-ai/kodegen.git
cd kodegen

# 2. Provision certificate (one-time)
cargo run --package kodegen_sign -- --interactive

# 3. Build helper app
cargo run --package kodegen_sign -- --build-helper

# 4. Verify output
ls -lh target/helper/
```

### Testing

```bash
# Run with verbose output
cargo run --package kodegen_sign -- --interactive --verbose

# Dry-run mode (validate without changes)
cargo run --package kodegen_sign -- --config signing.toml --dry-run

# Show current configuration
cargo run --package kodegen_sign -- --show
```

### CI/CD Integration

**GitHub Actions Example**:

```yaml
name: Build Helper

on:
  push:
    branches: [main]
  release:
    types: [created]

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Provision Certificate
        env:
          ISSUER_ID: ${{ secrets.APP_STORE_CONNECT_ISSUER_ID }}
          KEY_ID: ${{ secrets.APP_STORE_CONNECT_KEY_ID }}
          PRIVATE_KEY: ${{ secrets.APP_STORE_CONNECT_PRIVATE_KEY }}
        run: |
          echo "$PRIVATE_KEY" > AuthKey.p8
          cargo run --package kodegen_sign -- \
            --issuer-id "$ISSUER_ID" \
            --key-id "$KEY_ID" \
            --private-key AuthKey.p8
            
      - name: Build Helper
        run: |
          cargo run --package kodegen_sign -- --build-helper
          
      - name: Upload to Release
        if: github.event_name == 'release'
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          cargo run --package kodegen_sign -- \
            --build-helper \
            --upload
```

## License

See `LICENSE.md` in the repository root.

## Contributing

See `CONTRIBUTING.md` in the repository root.

