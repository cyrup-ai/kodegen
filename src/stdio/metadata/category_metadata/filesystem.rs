//! Filesystem tools: read, write, search, edit files and directories

use kodegen_mcp_schema::filesystem;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn filesystem_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: filesystem::FS_CREATE_DIRECTORY,
            category: "filesystem",
            description: "Create a new directory or ensure a directory exists. Can create multiple nested directories in one operation. Automatically validates paths.' } fn ...",
            schema: build_schema::<filesystem::FsCreateDirectoryArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_DELETE_DIRECTORY,
            category: "filesystem",
            description: "Delete a directory and all its contents recursively. This operation is permanent and cannot be undone. Requires recursive=true to confirm deletion....",
            schema: build_schema::<filesystem::FsDeleteDirectoryArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_DELETE_FILE,
            category: "filesystem",
            description: "Delete a file from the filesystem. This operation is permanent and cannot be undone. Only deletes files, not directories. Automatically validates p...",
            schema: build_schema::<filesystem::FsDeleteFileArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_EDIT_BLOCK,
            category: "filesystem",
            description: "Apply surgical text replacements to files. Takes old_string and new_string, and performs exact string replacement. By default replaces one occurren...",
            schema: build_schema::<filesystem::FsEditBlockArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_GET_FILE_INFO,
            category: "filesystem",
            description: "Retrieve detailed metadata about a file or directory including size, creation time, last modified time, permissions, type, and line count (for text...",
            schema: build_schema::<filesystem::FsGetFileInfoArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_LIST_DIRECTORY,
            category: "filesystem",
            description: "List all files and directories in a specified path. Returns entries prefixed with [DIR] or [FILE] to distinguish types. Supports filtering hidden f...",
            schema: build_schema::<filesystem::FsListDirectoryArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_MOVE_FILE,
            category: "filesystem",
            description: "Move or rename files and directories. Can move files between directories and rename them in a single operation. Both source and destination must be...",
            schema: build_schema::<filesystem::FsMoveFileArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_READ_FILE,
            category: "filesystem",
            description: "Read the contents of a file from the filesystem or a URL. Supports text files (returned as text) and image files (returned as base64). Use offset a...",
            schema: build_schema::<filesystem::FsReadFileArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_READ_MULTIPLE_FILES,
            category: "filesystem",
            description: "Read multiple files in parallel. Returns results for all files, including errors for individual files that fail. Supports offset and length paramet...",
            schema: build_schema::<filesystem::FsReadMultipleFilesArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_SEARCH,
            category: "filesystem",
            description: "🚀 BLAZING-FAST SEARCH (10-100x faster than grep). Respects .gitignore automatically. Built on ripgrep.nn QUICK START:n • Find files: fs_search(patt...",
            schema: build_schema::<filesystem::FsSearchArgs>(),
        },
        ToolMetadata {
            name: filesystem::FS_WRITE_FILE,
            category: "filesystem",
            description: "Write or append to file contents. Supports two modes: 'rewrite' (overwrite entire file) and 'append' (add to end of file). Automatically validates ...",
            schema: build_schema::<filesystem::FsWriteFileArgs>(),
        },
    ]
}
