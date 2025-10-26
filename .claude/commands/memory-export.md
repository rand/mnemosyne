---
name: memory-export
description: Export memories to markdown for review and backup
---

I will export memories from Mnemosyne to a markdown file for review, backup, or sharing.

**Usage**:
- `/memory-export` - Export current project to memories-<project>.md
- `/memory-export --namespace <ns>` - Export specific namespace
- `/memory-export --output <file>` - Export to custom file
- `/memory-export --format <markdown|json>` - Export format (default: markdown)
- `/memory-export --all` - Export all namespaces

**Instructions for me**:

1. **Parse the arguments**:
   - Extract `--namespace` flag if present (otherwise auto-detect)
   - Extract `--output` flag if present
   - Extract `--format` flag if present (default: "markdown")
   - Extract `--all` flag if present (exports all namespaces)

2. **Determine namespace**:
   - If `--all` flag: set namespace to `null` (all)
   - If `--namespace` specified: use that
   - Otherwise auto-detect from git root + CLAUDE.md

3. **Fetch memories** using MCP:
   ```json
   {
     "name": "mnemosyne.list",
     "arguments": {
       "namespace": "<namespace or null>",
       "limit": 1000,
       "sort_by": "importance"
     }
   }
   ```

4. **Determine output filename**:
   - If `--output` specified: use that
   - Otherwise: `memories-<project-name>-<date>.md`
   - Example: `memories-ecommerce-2025-10-26.md`

5. **Generate markdown content**:
   ```markdown
   # Memory Export - <Namespace>
   Generated: <date and time>
   Total Memories: <count>

   ## Table of Contents
   - [Architecture Decisions](#architecture-decisions) (<count>)
   - [Code Patterns](#code-patterns) (<count>)
   - [Bug Fixes](#bug-fixes) (<count>)
   - [Configurations](#configurations) (<count>)
   - [Constraints](#constraints) (<count>)
   - [Entities](#entities) (<count>)
   - [Insights](#insights) (<count>)
   - [References](#references) (<count>)
   - [Preferences](#preferences) (<count>)

   ---

   ## Architecture Decisions

   ### [<importance>/10] <summary>
   **ID**: `<memory_id>`
   **Date**: <created_at>
   **Tags**: <tags>
   **Context**: <context>

   <full content>

   **Related Files**:
   - <file>
   - <file>

   **Links**:
   - → <linked memory summary> (<link_type>, strength: <strength>)

   **Metadata**:
   - Importance: <importance>/10
   - Confidence: <confidence>
   - Access count: <access_count>
   - Last accessed: <last_accessed_at>

   ---

   <repeat for each memory, grouped by type>
   ```

6. **Generate JSON format** (if `--format json`):
   ```json
   {
     "export_date": "<ISO 8601>",
     "namespace": "<namespace>",
     "total_memories": <count>,
     "memories": [
       {
         "id": "<id>",
         "summary": "<summary>",
         "content": "<content>",
         "memory_type": "<type>",
         "importance": <importance>,
         ...full memory object...
       }
     ]
   }
   ```

7. **Write to file**:
   - Use Write tool to create the file
   - Confirm to user: "✓ Exported <count> memories to <filename>"

8. **Format the confirmation**:
   ```
   ✓ Exported <count> memories to <filename>

   Breakdown by type:
   - Architecture Decisions: <count>
   - Code Patterns: <count>
   - Bug Fixes: <count>
   - Configurations: <count>
   - Constraints: <count>
   - Entities: <count>
   - Insights: <count>
   - References: <count>
   - Preferences: <count>

   File size: <size> KB
   Format: <markdown|json>

   You can now review, edit, or share this export file.
   To re-import (future feature): mnemosyne import <filename>
   ```

9. **Error handling**:
   - If MCP server not available: "Error: Mnemosyne MCP server not running"
   - If no memories found: "No memories found for export"
   - If file write fails: "Error: Could not write to <filename>: <reason>"

Please export the memories with the provided options.
