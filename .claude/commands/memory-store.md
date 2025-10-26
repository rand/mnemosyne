---
name: memory-store
description: Store a new memory in Mnemosyne with LLM enrichment
---

I will help you store a memory in Mnemosyne. Please provide the content you want to store as arguments to this command.

**Usage**:
- `/memory-store <content>` - Store with default importance (5)
- `/memory-store --importance <1-10> <content>` - Store with specific importance
- `/memory-store --context <context> <content>` - Store with additional context

**Instructions for me**:

1. **Parse the arguments**:
   - Extract `--importance` flag if present (default: 5)
   - Extract `--context` flag if present (default: "User-provided memory")
   - The remaining text is the memory content

2. **Auto-detect namespace**:
   - Use the Bash tool to detect the current project
   - Check for git root: `git rev-parse --show-toplevel 2>/dev/null`
   - If in a git repo, read `.claude/CLAUDE.md` or `CLAUDE.md` for project name
   - Parse YAML frontmatter for `project:` field, or use first H1 heading
   - Construct namespace as `project:<name>`
   - If no project detected, use `global`

3. **Call Mnemosyne MCP tool**:
   ```json
   {
     "name": "mnemosyne.remember",
     "arguments": {
       "content": "<parsed content>",
       "namespace": "<detected namespace>",
       "importance": <parsed importance>,
       "context": "<parsed context>"
     }
   }
   ```

4. **Format the output**:
   ```
   ✓ Memory stored successfully

   ID: <memory_id>
   Summary: <llm-generated summary>
   Tags: <comma-separated tags>
   Importance: <importance>/10
   Namespace: <namespace>
   ```

5. **Error handling**:
   - If MCP server not available: "Error: Mnemosyne MCP server not running. Start with 'mnemosyne serve'"
   - If API key not configured: "Error: Anthropic API key not set. Configure with 'mnemosyne config set-key'"
   - If content is empty: "Error: No content provided. Usage: /memory-store <content>"

Please proceed to store the memory with the arguments I provided.
