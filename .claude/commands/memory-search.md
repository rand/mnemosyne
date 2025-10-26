---
name: memory-search
description: Search memories using hybrid search (keyword + graph)
---

I will search Mnemosyne memories and display formatted results.

**Usage**:
- `/memory-search <query>` - Search with defaults
- `/memory-search --namespace <ns> <query>` - Filter by namespace
- `/memory-search --min-importance <1-10> <query>` - Filter by minimum importance
- `/memory-search --limit <N> <query>` - Limit number of results
- `/memory-search --no-graph <query>` - Disable graph expansion

**Instructions for me**:

1. **Parse the arguments**:
   - Extract `--namespace` flag if present (otherwise auto-detect)
   - Extract `--min-importance` flag if present
   - Extract `--limit` flag if present (default: 10)
   - Extract `--no-graph` flag if present (default: expand_graph = true)
   - The remaining text is the search query

2. **Auto-detect namespace** (if not specified):
   - Use Bash to check git root: `git rev-parse --show-toplevel 2>/dev/null`
   - Read CLAUDE.md for project name
   - Construct namespace as `project:<name>`
   - If no project: search all (`null` namespace)

3. **Call Mnemosyne MCP tool**:
   ```json
   {
     "name": "mnemosyne.recall",
     "arguments": {
       "query": "<search query>",
       "namespace": "<namespace or null>",
       "max_results": <limit>,
       "min_importance": <min_importance or null>,
       "expand_graph": <true/false>
     }
   }
   ```

4. **Format the output**:
   For each result, display:
   ```
   Found <count> memories matching "<query>":

   <for each result, numbered>:

   <number>. [<importance stars>/10] <memory_type>
      Created: <date>
      Summary: <summary>
      Match: <match_reason> (score: <score>)

      <content preview (first 200 chars)>...
      Tags: <tags>

   ```

   Where:
   - Importance stars: ⭐ repeated <importance> times
   - Memory type: ArchitectureDecision, CodePattern, BugFix, etc.
   - Date: formatted as YYYY-MM-DD
   - Match reason: from the search result
   - Score: formatted to 2 decimal places

   If no results:
   ```
   No memories found matching "<query>"

   Suggestions:
   - Try different keywords
   - Remove namespace filter to search globally
   - Lower min-importance threshold
   - Check if memories exist: /memory-list
   ```

5. **Error handling**:
   - If MCP server not available: "Error: Mnemosyne MCP server not running"
   - If query is empty: "Error: No search query provided. Usage: /memory-search <query>"

Please search for memories using the provided arguments.
