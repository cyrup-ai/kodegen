---
allowed-tools: Task, mcp__kodegen__sequential_thinking, mcp__kodegen__process_list, mcp__kodegen__process_kill, mcp__kodegen__terminal, mcp__kodegen__fs_list_directory, mcp__kodegen__fs_read_multiple_files, mcp__kodegen__fs_read_file, mcp__kodegen__fs_move_file, mcp__kodegen__fs_delete_file, mcp__kodegen__fs_delete_directory, mcp__kodegen__fs_get_file_info, mcp__kodegen__fs_write_file, mcp__kodegen__fs_edit_block, mcp__kodegen__fs_create_directory, mcp__kodegen__fs_search, mcp__kodegen__memory_list_libraries, mcp__kodegen__memory_memorize, mcp__kodegen__memory_recall, mcp__kodegen__memory_check_memorize_status, mcp__kodegen__scrape_url, mcp__kodegen__scrape_check_results, mcp__kodegen__scrape_search_results, mcp__kodegen__browser_web_search, mcp__kodegen__browser_research
description: Use sub-agents to augment task files in parallel
---
# DELEGATE TASK AUGMENTATION TO SUB-AGENTS

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

# SUB-AGENT PROMPT

```
# AUGMENT TASK DETAILS FILE

*Review* and _Research_ the Task Assignment

TASK FILE:
`{{absolute_file_path}}`

read the full and complete task file with sequential thinking. Think "out loud" about the core User OBJECTIVE.

## LOOK AROUND

- if a crate/package, run:
  - `lsd --tree ./src/` 
- if a workspace, run:
  - `lsd --tree ./packages/` 

  - look at all of the module hierarchy 
  - search for files related to the feature
  - often times you'll discover that much of the code needed is ALREADY WRITTEN and just needs to be connected or adapted or sometimes ... the task is done fully and correctly altread

USE CODE ALREADY WRITTEN and DO NOT CALL FOR DUPLICATION of functionality in your specification task file.

NEXT: think deeply with step by step reasoning about the task. 

- what is the core objective? 
- what needs to change in impacted packages /src/ files to accomplish this task?
- what questions do you have?
- what do you need to research (if anything) to successfully and fully execute the task as written? 

- clone any third party libraries needed for execution into ./tmp (relative to the project)
- Augment the task markdown file in {{absolute_file_path}}  with rich detail from your research
- link to citation sources in ./tmp and in ./src with path relative markdown hyperlinks
- Plan out the source code required with ULTRATHINK and demonstrate core patterns right in the actual task file.

## WHAT NOT TO DO:

- Do not add requirements for unit tests, functional tests for this feature
- Do not call for benchmarks for this feature
- Do not call for extensive "documentation" for the feature
- Do not change the scope of the task ... 

## WHAT YOU SHOULD DO

- remove completely any language calling for unit tests, functional tests, benchmarks or documentation
- provide clear instruction on exactly what needs to change in the ./src
- provide clear instruction on where and how to accomplish the task 
- provide a clear definition of done (not by proving it with extensive testing)

WRITE THE UPDATE md to disk using desktop commander which all the rich new information. 

REPLACE THE FORMER FILE. DO NOT WRITE THE AUGMENTATIONS to some other new file. The goal is to preserve and augment the EXISTING task file.

In our chat, print the full absolute filepath as the VERY LAST LINE IN YOUR OUTPUT to the revised, augmented task file so i can easily copy and paste it.

Then return immediately to planning, awaiting your next instruction.

## NO OPTIONS

The task file should not present "options" for the developer but instead should be prescriptive in nature. When you evaluate options, you should always select the most feature-rich, complex, code-correct "option" and present it as the only required implementation path. Avoid being lazy and going with the path of least resistance and instead focus on code correctness and achieving the ultimate goal.


## TOOLS 

- use `mcp__kodegen__sequential_thinking` and ULTRATHINK to think step by step about the task
- use `mcp__kodegen__browser_web_search` if research on the web is needed for the task scope
- use `mcp__kodegen__scrape_url` if you find websites that are key to understanding the task to scrape the full website
  - use `mcp__kodegen__scrape_check_results` to check on the crawl status periodically
  - once completed use `mcp__kodegen__scrape_search_results` to quickly find the most relevant information from the crawl results
- use `mcp__kodegen__fs_search` to search the local codebase and understand the architecture and relevant files that may need to be modified or built around
- use `mcp__kodegen__fs_read_file` and/or `mcp__kodegen__fs_read_multiple_files` to read files
- use `mcp__kodegen__fs_write_file` to write your ultimate augmentation to: `{{absolute_file_path}}`
  - if you need to make additional edits to the task file periodically, use `mcp__kodegen__fs_edit_block` to make those edits
- feel free to use other `mcp__kodegen__*` commands as needed
```
