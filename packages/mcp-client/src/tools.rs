// Tool name constants for all 75 kodegen tools

// Filesystem tools (14)
pub const READ_FILE: &str = "read_file";
pub const READ_MULTIPLE_FILES: &str = "read_multiple_files";
pub const WRITE_FILE: &str = "write_file";
pub const MOVE_FILE: &str = "move_file";
pub const DELETE_FILE: &str = "delete_file";
pub const DELETE_DIRECTORY: &str = "delete_directory";
pub const LIST_DIRECTORY: &str = "list_directory";
pub const CREATE_DIRECTORY: &str = "create_directory";
pub const GET_FILE_INFO: &str = "get_file_info";
pub const EDIT_BLOCK: &str = "edit_block";
pub const START_SEARCH: &str = "start_search";
pub const GET_MORE_SEARCH_RESULTS: &str = "get_more_search_results";
pub const STOP_SEARCH: &str = "stop_search";
pub const LIST_SEARCHES: &str = "list_searches";

// Terminal tools (5)
pub const START_TERMINAL_COMMAND: &str = "start_terminal_command";
pub const READ_TERMINAL_OUTPUT: &str = "read_terminal_output";
pub const SEND_TERMINAL_INPUT: &str = "send_terminal_input";
pub const STOP_TERMINAL_COMMAND: &str = "stop_terminal_command";
pub const LIST_TERMINAL_COMMANDS: &str = "list_terminal_commands";

// Process tools (2)
pub const LIST_PROCESSES: &str = "list_processes";
pub const KILL_PROCESS: &str = "kill_process";

// Introspection tools (2)
pub const GET_USAGE_STATS: &str = "get_usage_stats";
pub const GET_RECENT_TOOL_CALLS: &str = "get_recent_tool_calls";

// Prompt tools (4)
pub const ADD_PROMPT: &str = "add_prompt";
pub const EDIT_PROMPT: &str = "edit_prompt";
pub const DELETE_PROMPT: &str = "delete_prompt";
pub const GET_PROMPT: &str = "get_prompt";

// Sequential thinking (1)
pub const SEQUENTIAL_THINKING: &str = "sequential_thinking";

// Claude agent tools (5)
pub const SPAWN_CLAUDE_AGENT: &str = "spawn_claude_agent";
pub const READ_CLAUDE_AGENT_OUTPUT: &str = "read_claude_agent_output";
pub const SEND_CLAUDE_AGENT_PROMPT: &str = "send_claude_agent_prompt";
pub const TERMINATE_CLAUDE_AGENT_SESSION: &str = "terminate_claude_agent_session";
pub const LIST_CLAUDE_AGENTS: &str = "list_claude_agents";

// Citescrape tools (4)
pub const START_CRAWL: &str = "start_crawl";
pub const GET_CRAWL_RESULTS: &str = "get_crawl_results";
pub const SEARCH_CRAWL_RESULTS: &str = "search_crawl_results";
pub const WEB_SEARCH: &str = "web_search";

// Git tools (20)
pub const GIT_INIT: &str = "git_init";
pub const GIT_OPEN: &str = "git_open";
pub const GIT_CLONE: &str = "git_clone";
pub const GIT_DISCOVER: &str = "git_discover";
pub const GIT_BRANCH_CREATE: &str = "git_branch_create";
pub const GIT_BRANCH_DELETE: &str = "git_branch_delete";
pub const GIT_BRANCH_LIST: &str = "git_branch_list";
pub const GIT_BRANCH_RENAME: &str = "git_branch_rename";
pub const GIT_COMMIT: &str = "git_commit";
pub const GIT_LOG: &str = "git_log";
pub const GIT_ADD: &str = "git_add";
pub const GIT_CHECKOUT: &str = "git_checkout";
pub const GIT_FETCH: &str = "git_fetch";
pub const GIT_MERGE: &str = "git_merge";
pub const GIT_WORKTREE_ADD: &str = "git_worktree_add";
pub const GIT_WORKTREE_REMOVE: &str = "git_worktree_remove";
pub const GIT_WORKTREE_LIST: &str = "git_worktree_list";
pub const GIT_WORKTREE_LOCK: &str = "git_worktree_lock";
pub const GIT_WORKTREE_UNLOCK: &str = "git_worktree_unlock";
pub const GIT_WORKTREE_PRUNE: &str = "git_worktree_prune";

// GitHub tools (25)
pub const CREATE_ISSUE: &str = "create_issue";
pub const GET_ISSUE: &str = "get_issue";
pub const LIST_ISSUES: &str = "list_issues";
pub const UPDATE_ISSUE: &str = "update_issue";
pub const SEARCH_ISSUES: &str = "search_issues";
pub const ADD_ISSUE_COMMENT: &str = "add_issue_comment";
pub const GET_ISSUE_COMMENTS: &str = "get_issue_comments";
pub const CREATE_PULL_REQUEST: &str = "create_pull_request";
pub const UPDATE_PULL_REQUEST: &str = "update_pull_request";
pub const MERGE_PULL_REQUEST: &str = "merge_pull_request";
pub const GET_PULL_REQUEST_STATUS: &str = "get_pull_request_status";
pub const GET_PULL_REQUEST_FILES: &str = "get_pull_request_files";
pub const GET_PULL_REQUEST_REVIEWS: &str = "get_pull_request_reviews";
pub const CREATE_PULL_REQUEST_REVIEW: &str = "create_pull_request_review";
pub const ADD_PULL_REQUEST_REVIEW_COMMENT: &str = "add_pull_request_review_comment";
pub const REQUEST_COPILOT_REVIEW: &str = "request_copilot_review";
pub const CREATE_REPOSITORY: &str = "create_repository";
pub const FORK_REPOSITORY: &str = "fork_repository";
pub const LIST_BRANCHES: &str = "list_branches";
pub const CREATE_BRANCH: &str = "create_branch";
pub const LIST_COMMITS: &str = "list_commits";
pub const GET_COMMIT: &str = "get_commit";
pub const SEARCH_CODE: &str = "search_code";
pub const SEARCH_REPOSITORIES: &str = "search_repositories";
pub const SEARCH_USERS: &str = "search_users";

// Config tools (2)
pub const GET_CONFIG: &str = "get_config";
pub const SET_CONFIG_VALUE: &str = "set_config_value";

// Database tools (7)
pub const LIST_SCHEMAS: &str = "list_schemas";
pub const LIST_TABLES: &str = "list_tables";
pub const GET_TABLE_SCHEMA: &str = "get_table_schema";
pub const GET_TABLE_INDEXES: &str = "get_table_indexes";
pub const GET_STORED_PROCEDURES: &str = "get_stored_procedures";
pub const EXECUTE_SQL: &str = "execute_sql";
pub const GET_POOL_STATS: &str = "get_pool_stats";
