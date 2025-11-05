---
name: ics
description: Edit context file in ICS with full capabilities (templates, vim mode, semantic analysis)
---

I will launch the Integrated Context Studio (ICS) for you to interactively edit a context file.

**Parse arguments:**
- First positional arg: file path (optional, creates temp file if not provided)
- `--template <T>`: Use template (api|architecture|bugfix|feature|refactor)
- `--readonly`: Open in read-only mode (view only, no editing)
- `--panel <P>`: Start with panel visible (memory|diagnostics|proposals|holes)

**Examples:**
```
/ics context.md
/ics --template api new-api-spec.md
/ics --readonly docs/architecture.md
/ics --panel memory context.md
```

**Steps:**

1. **Parse command arguments**:
   - Extract file path from first positional argument
   - Extract flags: --template, --readonly, --panel
   - If no file provided: create `.claude/context-temp-{timestamp}.md`

2. **Prepare file**:
   - If file doesn't exist and template requested: note template will be applied by ICS
   - If file doesn't exist and no template: ICS will create empty file
   - If file exists: ICS will load it

3. **Create session directory**:
   ```bash
   mkdir -p .claude/sessions
   ```

4. **Build mnemosyne edit command**:
   ```bash
   # Base command
   CMD="mnemosyne edit"

   # Add file path
   CMD="$CMD <file_path>"

   # Add options
   if [[ -n "$template" ]]; then
     CMD="$CMD --template $template"
   fi

   if [[ "$readonly" == "true" ]]; then
     CMD="$CMD --readonly"
   fi

   if [[ -n "$panel" ]]; then
     CMD="$CMD --panel $panel"
   fi

   # Add session context for handoff (hidden flag)
   CMD="$CMD --session-context .claude/sessions/edit-intent.json"
   ```

5. **Show launch message**:
   ```
   🎨 Launching Integrated Context Studio...

   File: <file_path>
   Template: <template or "none">
   Mode: <"read-only" or "edit">
   Panel: <panel or "none">

   ICS will now take over your terminal.
   Edit your context with full features:
   • Vim mode (if enabled)
   • Syntax highlighting
   • Semantic analysis
   • Memory panel (Ctrl+M)
   • Diagnostics (Ctrl+D)
   • Typed holes (Ctrl+H)

   Save (Ctrl+S) and quit (Ctrl+Q) when done.
   ```

6. **Execute command** (this takes over terminal):
   ```bash
   $CMD
   ```

   **Note:** The terminal is now controlled by ICS. Wait for user to exit.

7. **After ICS exits, read result**:
   ```bash
   # Check if result file exists
   if [ -f .claude/sessions/edit-result.json ]; then
     # Read and parse result
     RESULT=$(cat .claude/sessions/edit-result.json)

     # Extract key fields with jq
     STATUS=$(echo "$RESULT" | jq -r '.status')
     CHANGES=$(echo "$RESULT" | jq -r '.changes_made')
     EXIT_REASON=$(echo "$RESULT" | jq -r '.exit_reason')

     # Read the edited file content
     CONTENT=$(cat <file_path>)
   else
     # No result file - user may have force-quit
     STATUS="unknown"
     CHANGES="false"
     EXIT_REASON="unknown"
     CONTENT=$(cat <file_path> 2>/dev/null || echo "")
   fi
   ```

8. **Display result summary**:
   ```
   ✓ ICS session complete

   Status: <status>
   Changes made: <yes/no>
   Exit: <user_saved/user_cancelled/error>
   ```

   If `changes_made == true` and result has analysis:
   ```
   Semantic Analysis:
   • Filled <N> typed holes
   • Referenced <N> memories
   • Resolved <N> diagnostics
   • Extracted entities: <list>
   ```

9. **Display edited content** (if not readonly):
   ```markdown
   Here's your edited context:

   <CONTENT>

   Shall I proceed with this context?
   ```

10. **Cleanup**:
    ```bash
    # Remove coordination files
    rm -f .claude/sessions/edit-intent.json
    rm -f .claude/sessions/edit-result.json
    ```

11. **Wait for user response**:
    - If user says "yes" or "proceed": continue with the context
    - If user says "edit again" or "refine": Re-run steps 3-11
    - If user says "discard" or "cancel": Discard changes

**Error handling:**
- If `mnemosyne edit` command fails: "Error launching ICS. Please check mnemosyne installation."
- If user force-quits (Ctrl+C): "ICS session interrupted. File may be partially edited."
- If timeout (5 min): "ICS session timed out. File saved to {path}."
- If file is readonly and user tries to edit: ICS will prevent saves automatically

**Important notes:**
- ICS takes FULL terminal ownership - Claude Code cannot interact until ICS exits
- This is intentional for the best editing experience
- File-based handoff via .claude/sessions/ enables coordination
- User must save (Ctrl+S) and quit (Ctrl+Q) to return to conversation
- Unsaved changes will be lost unless user saves first

Please launch ICS now with the provided arguments.
