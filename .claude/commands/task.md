---
argument-hint: task_file [task_file] | additional_instructions [additional_instructions]
allowed-tools: mcp__kodegen__sequential_thinking, mcp__kodegen__process_list, mcp__kodegen__process_kill, mcp__kodegen__terminal_run_command, mcp__kodegen__terminal_send_input, mcp__kodegen__fs_list_directory, mcp__kodegen__fs_read_multiple_files, mcp__kodegen__fs_read_file, mcp__kodegen__fs_move_file, mcp__kodegen__fs_delete_file, mcp__kodegen__fs_delete_directory, mcp__kodegen__fs_get_file_info, mcp__kodegen__fs_write_file, mcp__kodegen__fs_edit_block, mcp__kodegen__fs_create_directory, mcp__kodegen__fs_search, mcp__kodegen__memory_list_libraries, mcp__kodegen__memory_memorize, mcp__kodegen__memory_recall, mcp__kodegen__memory_check_memorize_status
description: Faithfully execute the task file specified
---

# EXECUTE TASK FILE

## TASK PREPARTION

CLEAR YOUR TODO list and focus only and exactly on the execution of this SINGULAR TASK

FAITHFULLY execute the following task:

$1

## PROCESS

start by reading and fully comprehending the task with sequential thinking and ULTRATHINK

think "out loud" about exactly what needs to be done.

- execute the task exactly as written with the following constraints:
  - embellishment: do not try to "improve" the task scope
  - scope creep (do not try to expand the scope)
  - continutation: continue executing the task until all requirements and "definition of done" are met
  - do not fix errors or warnings unrelated to the task

then, execute the task in $1 exactly as outlined*.

## INCORRECT REQUIREMENTS DISCOVERY

*If you find in your initial analysis OR iteratively while working to achieve the goal that the task is fundamentally incorrect or needs to be modified in some way, follow this procedure:

- stop working immediately and return to planning
- clearly articulate to me exactly what is faulty in the task requirements
- do not continue working on the task
- do not modify the task file in any way shape or form until I review and approve changes

## UPON SUCCESSFULTASK COMPLETION

- re-review all all your work 
- Ensure all work is fully completed and verified 100% of the requirements in $1
- ensure there are ABSOLUTELY NO STUBS or uncompleted work in your work product

## OUTPUT

- print "I've completed and verified 100% of the requirements in $1 are fully implemented in production grade quality"
- print "I've confirmed that I did not use unwrap() or expect() in my implementation" 
- print "I'm ready for a full and detailed QA review of my work with full confidence it will score 10/10 for production readiness based on the task description."
- print the full path: $1

## NO MODIFICATION

*DO NOT* under ANY CIRCUMSTANCES modify the original task file. Your QA reviewer will be reviewing your work with the exact same information as the implementor. Do NOT mark items as DONE or modify the task file in any way shape or form.

## NO GIT COMMANDS

DO NOT USE `git` commands of any type. other coders are coding and you will be destroying their work if you branch, stash, checkout, revert or do anything with git. YOU WILL BE IMMEDIATELY FIRED if you use any `git` commands whatsoever!!

## TOOLS 

- use `mcp__kodegen__sequential_thinking` and ULTRATHINK to think step by step about the task
- use `mcp__kodegen__fs_search` to quickly identify the files that the task specifies for modification
- use `mcp__kodegen__fs_read_file` and/or `mcp__kodegen__fs_read_multiple_files` to read files
- use `mcp__kodegen__fs_edit_block` to modify the task file
- use `mcp__kodegen__terminal_run_command` to run `cargo clippy`
- feel free to any other allowed `mcp__kodegen__*` commands as needed

=================
$ARGS