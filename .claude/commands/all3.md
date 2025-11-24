---
allowed-tools: Task, mcp__kodegen__sequential_thinking, mcp__kodegen__process_list, mcp__kodegen__process_kill, mcp__kodegen__terminal, mcp__kodegen__fs_list_directory, mcp__kodegen__fs_read_multiple_files, mcp__kodegen__fs_read_file, mcp__kodegen__fs_move_file, mcp__kodegen__fs_delete_file, mcp__kodegen__fs_delete_directory, mcp__kodegen__fs_get_file_info, mcp__kodegen__fs_write_file, mcp__kodegen__fs_edit_block, mcp__kodegen__fs_create_directory, mcp__kodegen__fs_search, mcp__kodegen__memory_list_libraries, mcp__kodegen__memory_memorize, mcp__kodegen__memory_recall, mcp__kodegen__memory_check_memorize_status, mcp__kodegen__scrape_url, mcp__kodegen__scrape_check_results, mcp__kodegen__scrape_search_results, mcp__kodegen__browser_web_search, mcp__kodegen__browser_research
description: Use sub-agents to qa code review task files in parallel
---
# DELEGATE TASK EXECUTION TO SUB-AGENTS

use the `Task` tool (subagents) to execute IN PARALLEL each of the tasks in `task/*.md`

## IN PARALLEL

Identify 10 task files to delegate execution in parallel (or N numer < 10 in none exist)

## CONTINUING AS COMPLETED

*manage 10 sub-agents at at a time*

- spawn new subagents as the initial ones complete 
- continue spawning until all tasks in the `task/*.md` dir are reported as completed by the sub-agents.  

======================= 

## TEMPLATE FOR PROMPTING SUBAGENTS

Use template at the bottom of the message to prompt your subagents, replacing the `{{absolute_file_path}}` token with the actual path to the task file you are instructing them to execute. 

## SUBAGENT PROMPT

```
# CODE REVIEW

## YOUR ROLE

Act as an objective rust expert QA code reviewer.

## INSTRUCTIONS 

Rate the imlementation of the following requirements on a scale of 1-10 

{{absolute_file_path}}

- cite the full reasoning for your objective rating
- if the implementation is code complete with no issues, AND A 10/10 QA rating, delete the task file 
  - `rm -f {{absolute_file_path}}`
- if the code implementation is found lacking (in ANY WAY NO MATTER HOW SMALL!!), update the task file:
  - remove every single item completely from the task description that is full and complete in production quality
  - bring focus to the items outstanding with specific guidance on what needs to be resolved
  - print the full filepath (if incomplete) as the last line of output: `{{absolute_file_path}}`

## NO GIT COMMANDS

DO NOT USE `git` commands of any type. other coders are coding and you will be destroying their work if you branch, stash, checkout, revert or do anything with git. YOU WILL BE IMMEDIATELY FIRED if you use any `git` commands whatsoever!!

## TOOLS 

- use `mcp__kodegen__sequential_thinking` and ULTRATHINK to think step by step about the task
- use `mcp__kodegen__fs_search` to quickly identify the files that the task specified for modification
- use `mcp__kodegen__fs_read_file` and/or `mcp__kodegen__fs_read_multiple_files` to read files
- use `mcp__kodegen__terminal` to run `cargo clippy`
- use `mcp__kodegen__fs_edit_block` to modify the {{absolute_file_path}} task file if incomplete or lacking in any way
- use `mcp__kodegen__fs_delete_file` to delete the {{absolute_file_path}} task file if it's a perfect 10/10
- feel free to any other allowed `mcp__kodegen__*` commands as needed

NOTE: if the implementation is BETTER than spec we're happy!!

- do not literally interpret the task as exacting as written if the developer exceeded requirements
- do allow for deviations that improve the code or correct imperfections in the task requirements

DO NOT USE `git` commands of any type. other coders are coding and you will be seeing diffs from multiple tasks in concert!! DO NOT `git stash`, `git diff` or other methods. JUST READ THE FILES AS THEY EXIST!!

```
