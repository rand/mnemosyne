---
name: memory-context
description: Load relevant project context from memories
---

I will load and display relevant project context from Mnemosyne to help orient you to the current project state.

**Usage**:
- `/memory-context` - Load context for current project
- `/memory-context <project-name>` - Load context for specific project
- `/memory-context --recent <days>` - Only recent memories
- `/memory-context --important` - Only high-importance memories (8+)

**Instructions for me**:

1. **Parse the arguments**:
   - Extract `<project-name>` if provided as positional arg
   - Extract `--recent <days>` flag if present
   - Extract `--important` flag if present

2. **Determine namespace**:
   - If `<project-name>` provided: use `project:<project-name>`
   - Otherwise auto-detect from git root + CLAUDE.md
   - Use Bash: `git rev-parse --show-toplevel 2>/dev/null`
   - Parse CLAUDE.md for project name
   - If no project: use `global`

3. **Fetch recent and important memories**:
   Call `mnemosyne.list` to get:
   - Recent memories (last 7 days or `--recent` days)
   - High-importance memories (importance >= 8 or `--important`)

   ```json
   {
     "name": "mnemosyne.list",
     "arguments": {
       "namespace": "<namespace>",
       "limit": 50,
       "sort_by": "recent"
     }
   }
   ```

4. **Build memory graph** (optional, for connected decisions):
   If there are high-importance memories, call `mnemosyne.graph` to see relationships:
   ```json
   {
     "name": "mnemosyne.graph",
     "arguments": {
       "seed_ids": [<top 5 important memory IDs>],
       "max_hops": 2
     }
   }
   ```

5. **Format the output**:
   ```
   📚 Loading context for project: <project_name>

   ## Recent Activity (last <N> days):
   - <date>: <summary>
   - <date>: <summary>
   ...

   ## Critical Decisions (importance 8+):
   - <summary> (<type>)
   - <summary> (<type>)
   ...

   ## Active Constraints:
   - <constraint content>
   - <constraint content>
   ...

   ## Key Patterns:
   - <code_pattern summary>
   - <code_pattern summary>
   ...

   ## Related Files (most mentioned):
   - <file_path>
   - <file_path>
   ...

   ## Memory Statistics:
   - Total memories: <count>
   - High importance (8+): <count>
   - Recent (7 days): <count>
   - Most connected: "<summary>" (<link_count> links)

   ---
   Use /memory-search <query> to find specific information
   Use /memory-list to browse all memories
   ```

6. **Error handling**:
   - If MCP server not available: "Error: Mnemosyne MCP server not running"
   - If no memories found: "No memories found for project: <project>. Start storing with /memory-store"
   - If project not detected: "No project detected. Showing global memories."

Please load and display the project context.
