---
name: memory-consolidate
description: Review and consolidate similar or duplicate memories
---

I will help you consolidate similar or duplicate memories to keep your knowledge base clean and organized.

**Usage**:
- `/memory-consolidate` - Find and review consolidation candidates
- `/memory-consolidate --auto` - Auto-apply all recommendations
- `/memory-consolidate <id1> <id2>` - Analyze specific pair
- `/memory-consolidate --namespace <ns>` - Limit to namespace

**Instructions for me**:

1. **Parse the arguments**:
   - Extract `--auto` flag if present (auto-apply mode)
   - Extract `--namespace` flag if present (otherwise auto-detect)
   - Extract two memory IDs if provided as positional args
   - If both `--auto` and IDs provided: error

2. **Auto-detect namespace** (if not specified):
   - Use Bash: `git rev-parse --show-toplevel 2>/dev/null`
   - Read CLAUDE.md for project name
   - Construct namespace as `project:<name>`
   - If no project: use `null` (all)

3. **Mode A: Specific pair analysis** (if two IDs provided):
   - Call `mnemosyne.consolidate` with the two IDs:
   ```json
   {
     "name": "mnemosyne.consolidate",
     "arguments": {
       "memory_ids": ["<id1>", "<id2>"],
       "auto_apply": false
     }
   }
   ```

   - Display the recommendation:
   ```
   Analyzing consolidation of two memories:

   Memory A [<importance>/10]: <summary>
   Created: <date>
   <content preview (200 chars)>

   Memory B [<importance>/10]: <summary>
   Created: <date>
   <content preview (200 chars)>

   ---

   LLM Recommendation: <MERGE|SUPERSEDE|KEEP_BOTH>

   <if MERGE>:
   Reason: Memories contain very similar information
   Action: Merge into Memory <A|B> (higher importance), archive the other
   New content will combine both perspectives

   <if SUPERSEDE>:
   Reason: Memory <A|B> contains updated/more accurate information
   Action: Keep Memory <kept>, mark Memory <superseded> as superseded

   <if KEEP_BOTH>:
   Reason: Memories are distinct and should be maintained separately
   Action: No consolidation needed

   ---

   Apply this recommendation? [y/N]:
   (Or run with /memory-consolidate --auto <id1> <id2> to apply automatically)
   ```

4. **Mode B: Find candidates** (default mode):
   - Call `mnemosyne.consolidate` without IDs:
   ```json
   {
     "name": "mnemosyne.consolidate",
     "arguments": {
       "namespace": "<namespace or null>",
       "auto_apply": <--auto flag value>
     }
   }
   ```

   - If candidates found, display each:
   ```
   🔍 Scanning for consolidation candidates in <namespace>...

   Found <N> candidate pairs:

   <for each pair>:

   <number>. <MERGE|SUPERSEDE|KEEP_BOTH> Recommended
      Memory A [<imp>/10]: "<summary>"
      Created: <date>, Tags: <tags>

      Memory B [<imp>/10]: "<summary>"
      Created: <date>, Tags: <tags>

      Similarity: <high|medium|low>
      Reason: <LLM reasoning>

      <if --auto>:
      ✓ Applied: <action taken>

      <if not --auto>:
      [View details: /memory-consolidate <idA> <idB>]

   ---
   ```

   - Summary at end:
   ```
   Summary:
   - Total pairs analyzed: <N>
   - Merge recommended: <N>
   - Supersede recommended: <N>
   - Keep both: <N>

   <if --auto>:
   - Actions applied: <N>
   - Memories archived: <N>

   <if not --auto>:
   To apply recommendations:
   - Review each pair: /memory-consolidate <id1> <id2>
   - Auto-apply all: /memory-consolidate --auto
   ```

5. **Interactive confirmation** (if not --auto):
   - After showing recommendation for a specific pair
   - Ask user: "Apply this recommendation? [y/N]: "
   - If 'y': Call consolidate again with `auto_apply: true`
   - If 'N' or anything else: Do nothing, exit

6. **Format the detailed pair view**:
   When showing specific pair details, include:
   - Full content (not just preview)
   - All tags and keywords
   - Related files and entities
   - Link information
   - Access statistics

7. **Error handling**:
   - If MCP server not available: "Error: Mnemosyne MCP server not running"
   - If API key not configured: "Error: Consolidation requires LLM. Configure API key with 'mnemosyne config set-key'"
   - If invalid memory IDs: "Error: Invalid memory ID(s). Use /memory-list to see available memories"
   - If no candidates found: "No consolidation candidates found. Your memory base is well-organized!"
   - If both --auto and IDs provided: "Error: Cannot use --auto with specific memory IDs"

8. **Safety checks**:
   - Never auto-consolidate memories with importance 9+ without confirmation
   - Warn if consolidating across different namespaces
   - Preserve all links when consolidating
   - Create audit trail of consolidations

Please proceed with consolidation analysis using the provided options.
