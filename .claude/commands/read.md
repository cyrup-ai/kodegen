---
allowed-tools: mcp__kodegen__sequential_thinking, mcp__kodegen__process_list, mcp__kodegen__process_kill,mcp__kodegen__terminal_start_command,mcp__kodegen__terminal_list_commands,mcp__kodegen__terminal_send_input,mcp__kodegen__terminal_read_output,mcp__kodegen__terminal_stop_command,mcp__kodegen__fs_list_directory, mcp__kodegen__fs_read_multiple_files, mcp__kodegen__fs_read_file,mcp__kodegen__fs_move_file, mcp__kodegen__fs_delete_file, mcp__kodegen__fs_delete_directory, mcp__kodegen__fs_get_file_info, mcp__kodegen__fs_write_file, mcp__kodegen__fs_move_file,mcp__kodegen__fs_edit_block,mcp__kodegen__fs_start_search, mcp__kodegen__fs_get_search_results,mcp__kodegen__fs_list_searches,mcp__kodegen__fs_stop_search,mcp__kodegen__memory_list_libraries, mcp__kodegen__memory_memorize, mcp__kodegen__memory_recall,mcp__kodegen__memory_check_memorize_status,mcp__kodegen__scrape_url, mcp__kodegen__scrape_check_results,mcp__kodegen__scrape_search_results,mcp__kodegen__browser_web_search,mcp__kodegen__browser_start_research,mcp__kodegen__browser_list_research_sessions,mcp__kodegen__browser_get_research_result,mcp__kodegen__browser_get_research_status, mcp__kodegen__git_clone, mcp__kodegen__github_get_file_contents, mcp__kodegen__github_search_code, mcp__kodegen__github_search_repositories, mcp__kodegen__github_search_issues
description: Read 
---

# READ

read FULL FILES with `mcp__kodegen__sequential_thinking` and KNOW how it works because you READ THE CODE. do not use conjecture to solve this problem. base your recommendations and planning on CERTAINTY and definitive, deterministic INFORMATION based on having READ and COMPREHENDED the context and based your plan on INFORMATION.

If third party libraries are essential to the solution:

- see if we already have sources in `./tmp` (project relative)
- see if we already have sources in `./docs` (project relative)
  - IF NOT use `mcp__kodegen__git_clone` to clone them into `./tmp/` (project relative) and explore the docs, examples and sources with sequential thinking.

## TOOLS 

- use `mcp__kodegen__sequential_thinking` and ULTRATHINK to think step by step about the task
- use `mcp__kodegen__browser_web_search` if research on the web is needed for the task scope
- use `mcp__kodegen__scrape_url` if you find websites that are key to understanding the task to scrape the full website
  - use `mcp__kodegen__scrape_check_results` to check on the crawl status periodically
  - once completed use `mcp__kodegen__scrape_search_results` to quickly find the most relevant information from the crawl results
- use `mcp__kodegen__fs_start_search` to search the local codebase and understand the architecture and relevant files that may need to be modified or built around
  - use `mcp__kodegen__fs_get_search_results` to check on the search status and view the results
- use `mcp__kodegen__fs_read_file` and/or `mcp__kodegen__fs_read_multiple_files` to read files
- use `mcp__kodegen__git_clone` to clone repositories into `./tmp` for research
- use these tools to explore github directly:
  - `mcp__kodegen__github_get_file_contents`
  - `mcp__kodegen__github_search_code`
  - `mcp__kodegen__github_search_repositories`
  - `mcp__kodegen__github_search_issues`
- feel free to any other allowed `mcp__kodegen__*` commands as needed

=================
==> KEY INSIGHT: be _CERTAIN_ BEFORE you code <==
$ARGS