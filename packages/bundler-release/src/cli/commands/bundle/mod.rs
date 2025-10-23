//! Bundle command implementation.
//!
//! Creates distributable packages for multiple platforms.

mod helpers;

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::{Result, ReleaseError, CliError};
use crate::git::{GitManager, GitConfig};
use crate::workspace::{WorkspaceInfo, SharedWorkspaceInfo};
use std::sync::Arc;

use helpers::{
    discover_binaries_from_workspace,
    create_bundler_settings,
    calculate_artifact_checksum,
    parse_package_type,
    print_bundle_summary,
    upload_bundles_to_github,
};

pub(crate) use helpers::build_workspace_binaries;

/// Execute bundle command
pub(super) async fn execute_bundle(args: &Args, config: &RuntimeConfig) -> Result<i32> {
    let Command::Bundle {
        no_build,
        release,
        rebuild_image,
        current_platform_only,
        platform,
        upload,
        name,
        version,
        target,
        github_repo,
        docker_memory,
        docker_memory_swap,
        docker_cpus,
        docker_pids_limit,
    } = &args.command else {
        unreachable!("execute_bundle called with non-Bundle command");
    };

    config.verbose_println("Starting bundle creation...");

    // 1. Analyze workspace
    let workspace: SharedWorkspaceInfo = Arc::new(WorkspaceInfo::analyze(&config.workspace_path)?);

    // 2. Discover binary crates
    let binaries = discover_binaries_from_workspace(&workspace)?;
    if binaries.is_empty() {
        return Err(ReleaseError::Cli(CliError::InvalidArguments {
            reason: "No binary crates found in workspace. Add [[bin]] sections to Cargo.toml".to_string(),
        }));
    }
    config.verbose_println(&format!("Found {} binar{}", binaries.len(), if binaries.len() == 1 { "y" } else { "ies" }));

    // 3. Build binaries by default (unless --no-build)
    if !*no_build {
        config.println("🔨 Building binaries in release mode...");
        build_workspace_binaries(&config.workspace_path, *release)?;
        config.success_println("Build complete");
    } else {
        config.verbose_println("Skipping build (--no-build specified)");
    }

    // 4. Configure bundler settings
    config.verbose_println("Configuring bundler...");
    let settings = create_bundler_settings(
        &workspace,
        &binaries,
        name,
        version,
        *release,
        target,
        platform.as_deref(),
    )?;

    // 5. Create bundler
    let bundler = crate::bundler::Bundler::new(settings)
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "bundler_init".to_string(),
            reason: e.to_string(),
        }))?;

    // 6. Determine platforms to bundle
    let requested_platforms = if let Some(platform_str) = platform {
        // Explicit platform specified - use only that one
        config.verbose_println(&format!("Bundling specific platform: {}", platform_str));
        vec![parse_package_type(platform_str)?]
    } else if *current_platform_only {
        // User explicitly requested current platform only
        let platforms = crate::bundler::PackageType::all_for_current_platform();
        config.verbose_println(&format!("Bundling current platform only: {} platform(s)", platforms.len()));
        platforms
    } else {
        // DEFAULT: All platforms across all OSes
        use crate::bundler::PackageType;
        config.println("📦 Bundling for all platforms (macOS, Linux, Windows)...");
        vec![
            PackageType::MacOsBundle,
            PackageType::Dmg,
            PackageType::Deb,
            PackageType::Rpm,
            PackageType::AppImage,
            PackageType::WindowsMsi,
            PackageType::Nsis,
        ]
    };

    // 7. Split platforms by execution environment (native vs container)
    let (native_platforms, container_platforms) =
        crate::cli::docker::split_platforms_by_host(&requested_platforms);

    let mut all_artifacts = Vec::new();

    // 8. Bundle native platforms locally
    if !native_platforms.is_empty() {
        config.println("📦 Bundling native platforms...");
        let artifacts = bundler.bundle_types(&native_platforms)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "bundler_execute_native".to_string(),
                reason: e.to_string(),
            }))?;
        all_artifacts.extend(artifacts);
    }

    // 9. Bundle containerized platforms in Docker
    if !container_platforms.is_empty() {
        config.println("📦 Bundling cross-platform packages in container...");

        // Check Docker availability
        crate::cli::docker::ContainerBundler::check_docker_available().await?;

        // Ensure builder image exists
        crate::cli::docker::ContainerBundler::ensure_image_built(&config.workspace_path, *rebuild_image, config).await?;

        // Create container bundler with resource limits
        let limits = if let Some(memory) = docker_memory {
            // User provided explicit limits
            crate::cli::docker::ContainerLimits::from_cli(
                memory.clone(),
                docker_memory_swap.clone(),
                docker_cpus.clone(),
                *docker_pids_limit,
            )
        } else {
            // Auto-detect safe limits
            crate::cli::docker::ContainerLimits::default()
        };

        let container = crate::cli::docker::ContainerBundler::with_limits(
            config.workspace_path.clone(),
            limits
        );

        // Log resource limits for transparency
        config.verbose_println(&format!(
            "Docker resource limits: {} memory, {} swap, {} CPUs, {} max processes",
            container.limits.memory,
            container.limits.memory_swap,
            container.limits.cpus,
            container.limits.pids_limit
        ));

        // Bundle each platform in container
        for platform in container_platforms {
            let paths = container.bundle_platform(platform, !*no_build, *release, config).await?;

            // Convert paths to BundledArtifact
            let size = paths.iter().fold(0u64, |acc, p| {
                acc + std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
            });

            let checksum = if !paths.is_empty() {
                calculate_artifact_checksum(&paths[0])?
            } else {
                String::new()
            };

            all_artifacts.push(crate::bundler::BundledArtifact {
                package_type: platform,
                paths,
                size,
                checksum,
            });
        }
    }

    // 10. Print summary
    print_bundle_summary(&all_artifacts, config);

    // 11. Upload to GitHub if requested
    if *upload {
        // Create GitManager to detect GitHub repo from git remote origin
        let git_config = GitConfig::default();
        let git_manager = GitManager::with_config(&config.workspace_path, git_config).await?;

        upload_bundles_to_github(&workspace, &all_artifacts, github_repo.as_deref(), &git_manager, config).await?;
    }

    Ok(0)
}
