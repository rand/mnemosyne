# End-to-End Test Suite

**Purpose**: Validate complete workflows for both human and agent users of Mnemosyne

**Test Categories**:
1. **Human Workflows**: Typical user interactions via CLI and slash commands
2. **Agent Workflows**: AI agent interactions for context preservation across sessions
3. **MCP Protocol**: Server-client communication via MCP tools

---

## Prerequisites

### Required Setup

1. **Build Mnemosyne**:
   ```bash
   cargo build --release
   ```

2. **Configure API Key**:
   ```bash
   cargo run -- config set-key <your-anthropic-api-key>
   # OR
   export ANTHROPIC_API_KEY=<your-key>
   ```

3. **Start MCP Server** (for slash command tests):
   ```bash
   cargo run -- serve &
   # Server will run on stdio, communicating via JSON-RPC 2.0
   ```

4. **Test Database**:
   ```bash
   # Tests use in-memory SQLite by default
   # OR specify test database:
   export DATABASE_URL=sqlite:/tmp/mnemosyne_test.db
   ```

---

## Test Execution

### Run All E2E Tests

```bash
# Human workflows (manual execution recommended)
./tests/e2e/run_human_workflows.sh

# Agent workflows (manual execution recommended)
./tests/e2e/run_agent_workflows.sh

# MCP protocol tests (automated)
cargo test --test mcp_e2e_test
```

### Run Individual Scenarios

```bash
# Human Workflow 1: New Project Setup
./tests/e2e/human_workflow_1_new_project.sh

# Human Workflow 2: Memory Discovery & Reuse
./tests/e2e/human_workflow_2_discovery.sh

# Human Workflow 3: Knowledge Consolidation
./tests/e2e/human_workflow_3_consolidation.sh

# Agent Workflow 1: Phase Transitions
./tests/e2e/agent_workflow_1_phase_transitions.sh

# Agent Workflow 2: Cross-Session Memory
./tests/e2e/agent_workflow_2_cross_session.sh

# Agent Workflow 3: Multi-Agent Collaboration
./tests/e2e/agent_workflow_3_collaboration.sh
```

---

## Test Structure

### Human Workflow Tests

Location: `tests/e2e/human_workflow_*.sh`

Each test script:
1. Sets up test environment
2. Executes CLI commands in sequence
3. Validates output and state
4. Cleans up test data
5. Reports pass/fail

**Format**: Shell scripts that can be run manually or automated

### Agent Workflow Tests

Location: `tests/e2e/agent_workflow_*.sh`

Each test script:
1. Simulates agent behavior (capture decisions, preserve context)
2. Tests context preservation at phase boundaries
3. Tests memory recall across sessions
4. Validates namespace hierarchy and consolidation

**Format**: Shell scripts with commentary explaining agent behavior

### MCP Protocol Tests

Location: `tests/mcp_e2e_test.rs`

**Format**: Rust integration test that:
1. Spawns MCP server process
2. Communicates via stdio using JSON-RPC 2.0
3. Calls MCP tools (mnemosyne.remember, mnemosyne.recall, etc.)
4. Validates responses
5. Terminates server

---

## Success Criteria

### Human Workflows

**Workflow 1: New Project Setup**
- [ ] User can store initial architecture decisions
- [ ] LLM enrichment generates quality summaries and keywords
- [ ] Memories are retrievable via search
- [ ] Namespace correctly set to project:mnemosyne

**Workflow 2: Memory Discovery & Reuse**
- [ ] User can search for relevant past decisions
- [ ] Hybrid search (keyword + graph) returns accurate results
- [ ] Results ranked by relevance
- [ ] Search performance <200ms

**Workflow 3: Knowledge Consolidation**
- [ ] User can identify duplicate memories
- [ ] LLM provides accurate consolidation recommendations
- [ ] Memories merge correctly (content preserved, originals archived)
- [ ] Links redirect from archived to new memories

### Agent Workflows

**Workflow 1: Phase Transitions**
- [ ] Context preserved at each phase boundary
- [ ] Phase artifacts (decisions, typed holes, plans) stored in correct namespace
- [ ] Context recoverable in next phase

**Workflow 2: Cross-Session Memory**
- [ ] Session memories created with session:project:* namespace
- [ ] Work state checkpoint includes next steps and blockers
- [ ] New session can recall previous session state
- [ ] Consolidation merges session→project correctly

**Workflow 3: Multi-Agent Collaboration**
- [ ] Multiple agents can access shared project memories
- [ ] Typed holes enable parallel work without conflicts
- [ ] Graph traversal shows relationships between agent work
- [ ] No race conditions in memory storage

### MCP Protocol Tests

**Server Startup**
- [ ] Server starts without errors
- [ ] Server responds to JSON-RPC 2.0 requests
- [ ] Server validates tool arguments
- [ ] Server handles errors gracefully

**Tool: mnemosyne.remember**
- [ ] Accepts content, namespace, importance parameters
- [ ] Returns memory ID and enrichment results
- [ ] Stores memory in database
- [ ] LLM enrichment works correctly

**Tool: mnemosyne.recall**
- [ ] Accepts query, namespace, max_results parameters
- [ ] Returns ranked results with relevance scores
- [ ] Hybrid search (keyword + graph) works
- [ ] Performance <200ms

**Tool: mnemosyne.list**
- [ ] Accepts namespace, limit, sort_by parameters
- [ ] Returns memories sorted correctly
- [ ] Limit parameter enforced

**Tool: mnemosyne.consolidate**
- [ ] Identifies candidate pairs
- [ ] LLM provides consolidation decisions
- [ ] Auto-apply mode works correctly
- [ ] Audit trail created

**Tool: mnemosyne.graph**
- [ ] Returns memory graph from seed IDs
- [ ] Max hops parameter enforced
- [ ] Link information included
- [ ] Graph structure valid

**Error Handling**
- [ ] Invalid tool name returns proper error
- [ ] Missing required parameters returns validation error
- [ ] Database errors handled gracefully
- [ ] API key missing returns helpful error

---

## Validation Checklist

Before marking Phase 3 complete:

- [ ] All human workflow tests pass
- [ ] All agent workflow tests pass
- [ ] All MCP protocol tests pass
- [ ] Performance targets met (search <200ms, enrichment <2s)
- [ ] No critical bugs found
- [ ] Test coverage documented
- [ ] Gaps identified and documented in gap-analysis.md

---

## Known Limitations

1. **Manual Execution**: Some tests require manual observation (agent behavior)
2. **Real API Calls**: LLM enrichment tests make real API calls (costs money, requires key)
3. **Timing Variability**: Performance tests may vary based on network/API latency
4. **MCP Server**: Must be manually started for slash command tests

---

## Troubleshooting

### MCP Server Won't Start

```bash
# Check if already running
ps aux | grep mnemosyne

# Check logs
tail -f ~/.mnemosyne/logs/server.log

# Try with verbose logging
RUST_LOG=debug cargo run -- serve
```

### Tests Fail with "No API Key"

```bash
# Set API key
cargo run -- config set-key <key>

# OR use environment variable
export ANTHROPIC_API_KEY=<key>

# Verify key is set
cargo run -- config show-key
```

### Database Locked Errors

```bash
# Kill any running mnemosyne processes
pkill -9 mnemosyne

# Remove lock files
rm -f ~/.mnemosyne/mnemosyne.db-wal
rm -f ~/.mnemosyne/mnemosyne.db-shm

# Use in-memory database for tests
unset DATABASE_URL
```

### Search Returns No Results

```bash
# Check if memories exist
cargo run -- list

# Check namespace
cargo run -- list --namespace project:mnemosyne

# Try global search
cargo run -- search "<query>" --namespace global
```

---

## Reporting Issues

When a test fails:

1. **Capture full output**: Save stdout and stderr
2. **Document environment**: OS, Rust version, database type
3. **Reproduce minimally**: Isolate smallest failing case
4. **Add to gap-analysis.md**: Document as P0-P3 issue with:
   - Severity
   - Component
   - Impact
   - Steps to reproduce
   - Expected vs actual behavior

---

## Next Steps After Phase 3

1. Review all test results
2. Update gap-analysis.md with any issues found
3. Create remediation plan for P0-P1 issues
4. Proceed to Phase 4: Gap Analysis & Remediation
