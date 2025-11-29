---
argument-hint: directory [directory]
allowed-tools: mcp__kodegen__sequential_thinking, mcp__kodegen__process_list, mcp__kodegen__process_kill, mcp__kodegen__terminal, mcp__kodegen__fs_list_directory, mcp__kodegen__fs_read_multiple_files, mcp__kodegen__fs_read_file, mcp__kodegen__fs_move_file, mcp__kodegen__fs_delete_file, mcp__kodegen__fs_delete_directory, mcp__kodegen__fs_get_file_info, mcp__kodegen__fs_write_file, mcp__kodegen__fs_edit_block, mcp__kodegen__fs_create_directory, mcp__kodegen__fs_search, mcp__kodegen__memory_list_libraries, mcp__kodegen__memory_memorize, mcp__kodegen__memory_recall, mcp__kodegen__memory_check_memorize_status, mcp__kodegen__scrape_url, mcp__kodegen__browser_web_search, mcp__kodegen__browser_research
description: Perform a thorough and detailed code review
---

# CODE REVIEW

perform a thorough and detailed code review of this module:

`$1`

 identify any:

 - stubs (as in non-functional required code)
- non-production code or suboptimal code
- races
- performance bottlenecks
- logical issues
- other issues

create task files in the WORKSPACE ROOT

`{{WORKSPACE_ROOT}}/task/*.md` 
(one per item found for any issues identified)

with detailed notes on any issue identified no matter no small

DO NOT FOCUS ON:

- lack of test coverage
- lack of benchmarks 

DO FOCUS ON:

- runtime performance
- code clarity
- hidden errors
- real world issues in the product

## TOOLS 

- use `mcp__kodegen__sequential_thinking` and ULTRATHINK to think step by step about the code review
- use `mcp__kodegen__browser_web_search` if research on the web is needed for the code review
- use `mcp__kodegen__scrape_url` if you find websites that are key to understanding the code to scrape the full website
  - use `mcp__kodegen__scrape_check_results` to check on the crawl status periodically
  - once completed use `mcp__kodegen__scrape_search_results` to quickly find the most relevant information from the crawl results
- use `mcp__kodegen__fs_search` to search the local codebase and understand the architecture and relevant files that may need to be modified or built around
- use `mcp__kodegen__fs_read_file` and/or `mcp__kodegen__fs_read_multiple_files` to read files
- use `mcp__kodegen__fs_write_file` to write tasks for any issues discovered during review
  - if you need to make additional edits to the code review tasks periodically, use `mcp__kodegen__fs_edit_block` to make those edits
- feel free to use other `mcp__kodegen__*` commands as needed

=================
$ARGS
