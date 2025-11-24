---
argument-hint: task_file [task_file] | additional_instructions [additional_instructions]
allowed-tools: mcp__kodegen__sequential_thinking, mcp__kodegen__process_list, mcp__kodegen__process_kill, mcp__kodegen__terminal_run_command, mcp__kodegen__terminal_send_input, mcp__kodegen__fs_list_directory, mcp__kodegen__fs_read_multiple_files, mcp__kodegen__fs_read_file, mcp__kodegen__fs_move_file, mcp__kodegen__fs_delete_file, mcp__kodegen__fs_delete_directory, mcp__kodegen__fs_get_file_info, mcp__kodegen__fs_write_file, mcp__kodegen__fs_edit_block, mcp__kodegen__fs_create_directory, mcp__kodegen__fs_search, mcp__kodegen__memory_list_libraries, mcp__kodegen__memory_memorize, mcp__kodegen__memory_recall, mcp__kodegen__memory_check_memorize_status, mcp__kodegen__scrape_url, mcp__kodegen__scrape_check_results, mcp__kodegen__scrape_search_results, mcp__kodegen__browser_web_search, mcp__kodegen__browser_research
description: Decompose a large task into smaller, single-session tasks.
---

# DECOMPOSE A LARGE TASK

## SETUP

`mkdir -p ./task`
(workspace root)

## NAMING 

choose a short, all uppercase, human meaningful, less than 8 character task prefix. 

For example if the tasks are related to "in production" stubs you would name the tasks 

- `IN_PROD_1.md`, 
- `IN_PROD_2.md`
- `IN_PROD_3.md` 
- etc.

# TASK BREAKDOWN 

`{clipboard}`

^^ Convert this task/todo work file into a series of individual markdown task files: 

- ./task/TASK1.md 
- ./task/TASK2.md 
- ./task/TASK3.md 
... etc. ...

## EACH TASK FILE 

Each numbered task should be focused on a specific "area of focus". 

*Additionally:* 

- the tasks should be sparse and cleare
- each task should easily be achievable in **single Claude session**. 
- no tasks should call for "research" as a task (they will all be fully researched and documented prior to execution)
- NEVER create tasks for writing unit tests, integration tests or benchmarks. Focus solely on ./src modification and functionality 

## TASK FORMAT

Ensure the task clearly outlines:

- the `OBJECTIVE:` 
- Clear and actionable, numbered tasks SUBTASK1, SUBTASK2, SUBTASK3 etc.
 - Clear and well defined definitions of:
  - what needs to change
  - where the changes need to happen
  - why the changes need to happen 
- Concise definitions of the work required, and clear definitions of done
- Research notes
- locations to cloned documentation and third party source code required to successfully implement the feature

## AVOID "MEGA TASKS"

 - tasks should be comfortably achievable in one single, focused Claude session
- after you write each task file, ask yourself "is this really doable in one claude session?"
  - if no, rewrite it with A, B, C file variants, further subdividing the work

## NO TESTS

Really? Yes, really. There's another team that's responsible for writing tests. You'll be making their work harder if any test code is written. Add clear requirements to the task that no tests are to be written.

## NO BENCHES

There's another team that's responsible for writing benchmarks (if I want them and the product needs them). You'll be makign their work harder if any benchmark code is written. Add clear requirements to the task that no benches are to be written.

 ## DELETE the original when you're done 

`rm -f {clipboard}`

- do not move the original or back it up 
- delete it so it's not confused with the decomposed work