//! Data and persistence tools: database, filesystem

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn data_persistence_tools() -> Vec<ToolMetadata> {
    vec![
        // DATABASE (7 tools)
        ToolMetadata {
            name: "execute_sql",
            category: "database",
            description: "Execute SQL query or multiple SQL statements (separated by semicolons). nn MULTI-STATEMENT BEHAVIOR:n - Write operations (INSERT/UPDATE/DELETE/CREA...",
            schema: build_schema::<database::ExecuteSQLArgs>(),
        },
        ToolMetadata {
            name: "get_pool_stats",
            category: "database",
            description: "Get connection pool health metrics including active connections, idle connections, and pool configuration. Use this to diagnose connection pool exh...",
            schema: build_schema::<database::GetPoolStatsArgs>(),
        },
        ToolMetadata {
            name: "get_stored_procedures",
            category: "database",
            description: "List stored procedures in a schema. Returns procedure names and optionally detailed information including parameters and definitions. Not supported...",
            schema: build_schema::<database::GetStoredProceduresArgs>(),
        },
        ToolMetadata {
            name: "get_table_indexes",
            category: "database",
            description: "Get index information for a table including index names, columns, uniqueness, and primary key status. Use this to understand which columns are inde...",
            schema: build_schema::<database::GetTableIndexesArgs>(),
        },
        ToolMetadata {
            name: "get_table_schema",
            category: "database",
            description: "Get column information for a table including column names, data types, nullability, and default values. Use this before writing queries to understa...",
            schema: build_schema::<database::GetTableSchemaArgs>(),
        },
        ToolMetadata {
            name: "list_schemas",
            category: "database",
            description: "List all schemas (databases) in the current database connection. For PostgreSQL, returns all user schemas (excludes pg_catalog, information_schema)...",
            schema: build_schema::<database::ListSchemasArgs>(),
        },
        ToolMetadata {
            name: "list_tables",
            category: "database",
            description: "List all tables in a schema. If schema not provided, uses default schema (public for PostgreSQL, current database for MySQL, main for SQLite, dbo f...",
            schema: build_schema::<database::ListTablesArgs>(),
        },
        // FILESYSTEM (14 tools)
        ToolMetadata {
            name: "fs_create_directory",
            category: "filesystem",
            description: "Create a new directory or ensure a directory exists. Can create multiple nested directories in one operation. Automatically validates paths.' } fn ...",
            schema: build_schema::<filesystem::FsCreateDirectoryArgs>(),
        },
        ToolMetadata {
            name: "fs_delete_directory",
            category: "filesystem",
            description: "Delete a directory and all its contents recursively. This operation is permanent and cannot be undone. Requires recursive=true to confirm deletion....",
            schema: build_schema::<filesystem::FsDeleteDirectoryArgs>(),
        },
        ToolMetadata {
            name: "fs_delete_file",
            category: "filesystem",
            description: "Delete a file from the filesystem. This operation is permanent and cannot be undone. Only deletes files, not directories. Automatically validates p...",
            schema: build_schema::<filesystem::FsDeleteFileArgs>(),
        },
        ToolMetadata {
            name: "fs_edit_block",
            category: "filesystem",
            description: "Apply surgical text replacements to files. Takes old_string and new_string, and performs exact string replacement. By default replaces one occurren...",
            schema: build_schema::<filesystem::FsEditBlockArgs>(),
        },
        ToolMetadata {
            name: "fs_get_file_info",
            category: "filesystem",
            description: "Retrieve detailed metadata about a file or directory including size, creation time, last modified time, permissions, type, and line count (for text...",
            schema: build_schema::<filesystem::FsGetFileInfoArgs>(),
        },
        ToolMetadata {
            name: "fs_get_search_results",
            category: "filesystem",
            description: "Get more results from an active search with offset-based pagination.nn Supports partial result reading with:n - 'offset' (start result index, defau...",
            schema: build_schema::<filesystem::FsGetMoreSearchResultsArgs>(),
        },
        ToolMetadata {
            name: "fs_list_directory",
            category: "filesystem",
            description: "List all files and directories in a specified path. Returns entries prefixed with [DIR] or [FILE] to distinguish types. Supports filtering hidden f...",
            schema: build_schema::<filesystem::FsListDirectoryArgs>(),
        },
        ToolMetadata {
            name: "fs_list_searches",
            category: "filesystem",
            description: "List all active searches.nn Shows search IDs, search types, patterns, status, and runtime.n Similar to list_sessions for terminal processes. Useful...",
            schema: build_schema::<filesystem::FsListSearchesArgs>(),
        },
        ToolMetadata {
            name: "fs_move_file",
            category: "filesystem",
            description: "Move or rename files and directories. Can move files between directories and rename them in a single operation. Both source and destination must be...",
            schema: build_schema::<filesystem::FsMoveFileArgs>(),
        },
        ToolMetadata {
            name: "fs_read_file",
            category: "filesystem",
            description: "Read the contents of a file from the filesystem or a URL. Supports text files (returned as text) and image files (returned as base64). Use offset a...",
            schema: build_schema::<filesystem::FsReadFileArgs>(),
        },
        ToolMetadata {
            name: "fs_read_multiple_files",
            category: "filesystem",
            description: "Read multiple files in parallel. Returns results for all files, including errors for individual files that fail. Supports offset and length paramet...",
            schema: build_schema::<filesystem::FsReadMultipleFilesArgs>(),
        },
        ToolMetadata {
            name: "fs_start_search",
            category: "filesystem",
            description: "Start a streaming search that can return results progressively.nn SEARCH STRATEGY GUIDE:n Choose the right search type based on what the user is lo...",
            schema: build_schema::<filesystem::FsStartSearchArgs>(),
        },
        ToolMetadata {
            name: "fs_stop_search",
            category: "filesystem",
            description: "Stop an active search session.nn Stops the background search process gracefully. Use this when you've found what you need or if a search is taking ...",
            schema: build_schema::<filesystem::FsStopSearchArgs>(),
        },
        ToolMetadata {
            name: "fs_write_file",
            category: "filesystem",
            description: "Write or append to file contents. Supports two modes: 'rewrite' (overwrite entire file) and 'append' (add to end of file). Automatically validates ...",
            schema: build_schema::<filesystem::FsWriteFileArgs>(),
        },
    ]
}
