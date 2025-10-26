---
name: memory-list
description: List and browse memories with sorting and filtering
---

I will list memories from Mnemosyne with optional sorting and filtering.

**Usage**:
- `/memory-list` - List recent memories (default)
- `/memory-list --sort <recent|importance|access>` - Sort by criteria
- `/memory-list --type <type>` - Filter by memory type
- `/memory-list --namespace <ns>` - Filter by namespace
- `/memory-list --limit <N>` - Limit results (default: 20)

**Instructions for me**:

1. **Parse the arguments**:
   - Extract `--sort` flag if present (default: "recent")
   - Extract `--type` flag if present (for display filtering, not MCP)
   - Extract `--namespace` flag if present (otherwise auto-detect)
   - Extract `--limit` flag if present (default: 20)

2. **Auto-detect namespace** (if not specified):
   - Use Bash: `git rev-parse --show-toplevel 2>/dev/null`
   - Read CLAUDE.md for project name
   - Construct namespace as `project:<name>`
   - If no project: list all (`null` namespace)

3. **Call Mnemosyne MCP tool**:
   ```json
   {
     "name": "mnemosyne.list",
     "arguments": {
       "namespace": "<namespace or null>",
       "limit": <limit>,
       "sort_by": "<recent|importance|access_count>"
     }
   }
   ```

4. **Format the output as a table**:
   ```
   Memories (sorted by <sort_by>, limit <limit>):

    # | Date       | Type                | Imp | Summary
   ---|------------|---------------------|-----|--------------------------------------------
    1 | 2025-10-25 | Bug Fix            |  6  | Fixed race condition in order processing
    2 | 2025-10-24 | Configuration      |  5  | Updated webhook endpoint for payments
    3 | 2025-10-23 | Code Pattern       |  7  | Added retry logic with exponential backoff
    4 | 2025-10-22 | Architecture       |  9  | Event-driven order processing design
    5 | 2025-10-21 | Constraint         |  8  | All events must be idempotent
   ...

   <count> memories total (showing <shown>)

   Commands:
   - /memory-search <query> - Search for specific memories
   - /memory-export - Export all memories to markdown
   - /memory-context - Load project context
   ```

   **Formatting rules**:
   - Date: YYYY-MM-DD format
   - Type: Abbreviated if needed (e.g., "Arch Decision" for ArchitectureDecision)
   - Imp: Importance score 1-10
   - Summary: Truncate to ~40-50 chars if needed with "..."

5. **Apply client-side type filter** (if `--type` specified):
   - Filter results to only show matching memory_type
   - Update count accordingly

6. **Error handling**:
   - If MCP server not available: "Error: Mnemosyne MCP server not running"
   - If no memories found: "No memories found. Start storing with /memory-store <content>"

Please list the memories with the provided filters.
