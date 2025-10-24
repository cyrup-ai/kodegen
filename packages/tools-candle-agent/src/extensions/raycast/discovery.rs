use crate::extensions::common::types::{ExtensionError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaycastExtension {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub commands: Vec<RaycastCommand>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaycastCommand {
    pub name: String,
    pub title: String,
    pub mode: CommandMode,
    pub script_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandMode {
    View,
    NoView,
    FullOutput,
    Inline,
    Silent,
    Compact,
}

pub async fn discover_extensions() -> Result<Vec<RaycastExtension>> {
    let home_dir = dirs::home_dir().ok_or(ExtensionError::HomeDirectoryNotFound)?;

    let extensions_path = home_dir
        .join("Library")
        .join("Application Support")
        .join("com.raycast.macos")
        .join("extensions");

    // If Raycast not installed, return empty vec instead of error
    if !extensions_path.exists() {
        return Ok(Vec::new());
    }

    let mut extensions = Vec::new();
    let mut entries = fs::read_dir(&extensions_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            // Try to read package.json if it exists
            let package_json = path.join("package.json");
            if package_json.exists()
                && let Ok(extension) = parse_extension_metadata(&path, &package_json).await
            {
                extensions.push(extension);
            }
        }
    }

    Ok(extensions)
}

async fn parse_extension_metadata(
    extension_path: &Path,
    package_json: &PathBuf,
) -> Result<RaycastExtension> {
    let content = fs::read_to_string(package_json).await?;
    let package: serde_json::Value = serde_json::from_str(&content)?;

    let name = package
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let title = package
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&name)
        .to_string();

    let description = package
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let id = extension_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(RaycastExtension {
        id,
        name,
        title,
        description,
        commands: Vec::new(), // TypeScript extensions not directly executable
        path: extension_path.to_path_buf(),
    })
}

pub async fn discover_script_commands(paths: &[PathBuf]) -> Result<Vec<RaycastCommand>> {
    let mut commands = Vec::new();

    for path in paths {
        if !path.exists() {
            continue;
        }

        if path.is_file() {
            if let Ok(command) = parse_script_command(path).await {
                commands.push(command);
            }
        } else if path.is_dir() {
            let mut entries = fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let file_path = entry.path();
                if file_path.is_file()
                    && let Ok(command) = parse_script_command(&file_path).await
                {
                    commands.push(command);
                }
            }
        }
    }

    Ok(commands)
}

async fn parse_script_command(path: &PathBuf) -> Result<RaycastCommand> {
    let content = fs::read_to_string(path).await?;
    let lines: Vec<&str> = content.lines().collect();

    let mut title = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();
    let mut mode = CommandMode::FullOutput;

    for line in lines.iter().take(50) {
        if line.contains("@raycast.title")
            && let Some(pos) = line.find("@raycast.title")
        {
            let rest = line[pos + "@raycast.title".len()..].trim();
            title = rest.to_string();
        }
        if line.contains("@raycast.mode")
            && let Some(pos) = line.find("@raycast.mode")
        {
            let rest = line[pos + "@raycast.mode".len()..].trim();
            mode = match rest {
                "fullOutput" => CommandMode::FullOutput,
                "inline" => CommandMode::Inline,
                "silent" => CommandMode::Silent,
                "compact" => CommandMode::Compact,
                _ => CommandMode::FullOutput,
            };
        }
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(RaycastCommand {
        name,
        title,
        mode,
        script_path: Some(path.clone()),
    })
}
