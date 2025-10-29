# Troubleshooting Branch Isolation

## Common Issues and Solutions

### Issue: "Branch already has N agent(s) assigned in Isolated mode"

**Cause**: Another agent is working on this branch in isolated mode.

**Solutions**:

1. **Wait for agent to finish**:
   ```bash
   mnemosyne branch status --all
   # Check timeout remaining
   ```

2. **Request coordinated mode instead**:
   ```bash
   mnemosyne branch join <branch> <intent> --mode coordinated
   ```

3. **Work on different branch**:
   ```bash
   git checkout -b <branch>-alternate
   mnemosyne branch join <branch>-alternate full
   ```

4. **Check if assignment is stale**:
   ```bash
   # Stale processes are auto-cleaned after 30 seconds of no heartbeat
   # If process is truly dead, wait 30 seconds and retry
   ```

### Issue: Agent switched to wrong branch

**Cause**: Multiple agents operating without coordination, branch state confusion.

**Diagnosis**:
```bash
# Check current assignment
mnemosyne branch status

# Check git branch
git branch --show-current

# Check if assignment matches git branch
```

**Solution**:
```bash
# Release incorrect assignment
mnemosyne branch release

# Checkout correct branch
git checkout <correct-branch>

# Join with proper intent
mnemosyne branch join <correct-branch> full
```

**Prevention**:
- Always use `mnemosyne branch switch` instead of raw `git checkout`
- Enable branch validation in git hooks

### Issue: "Conflict detected" notification spam

**Cause**: Multiple agents modifying overlapping files.

**Solutions**:

1. **Partition work by files**:
   ```bash
   # Agent 1
   mnemosyne branch join main write --files "src/module_a/**"

   # Agent 2
   mnemosyne branch join main write --files "src/module_b/**"
   ```

2. **Sequential work instead of parallel**:
   ```bash
   # Agent 1 finishes first
   git commit && mnemosyne branch release

   # Agent 2 starts
   mnemosyne branch join main write --files src/shared.rs
   ```

3. **Adjust notification frequency**:
   ```toml
   # .mnemosyne/config.toml
   [notifications]
   periodic_interval_minutes = 60  # Reduce from default 20
   on_save = false                 # Disable on-save notifications
   ```

### Issue: Cross-process coordination not working

**Cause**: File permissions, stale lock files, or missing directory.

**Diagnosis**:
```bash
# Check mnemosyne directory exists
ls -la .mnemosyne/

# Check file permissions
ls -l .mnemosyne/*.json

# Check for stale lock files
find .mnemosyne/ -name "*.lock" -mmin +5
```

**Solution**:
```bash
# Clean up stale state
rm -rf .mnemosyne/coordination_queue/*.json
rm -rf .mnemosyne/*.lock

# Restart coordination
mnemosyne branch status
```

**Prevention**:
- Ensure `.mnemosyne/` is gitignored
- Use proper file locking (automatic on Unix)
- Enable process liveness detection

### Issue: Assignment timeout too short

**Cause**: Complex work requires more time than default timeout.

**Solutions**:

1. **Configure phase-specific multipliers**:
   ```toml
   # .mnemosyne/config.toml
   [branch_isolation.timeout_multipliers]
   plan_to_artifacts = 3.0  # Increase from default 2.0
   ```

2. **Send progress updates**:
   ```bash
   # Heartbeats extend timeout automatically
   # Ensure agent process is running and sending heartbeats
   ```

3. **Break work into smaller phases**:
   ```bash
   # Instead of one large assignment, use multiple smaller ones
   mnemosyne branch join main write --files src/part1.rs
   # Complete work
   mnemosyne branch release
   mnemosyne branch join main write --files src/part2.rs
   ```

### Issue: "Permission denied" errors

**Cause**: Orchestrator bypass not working or incorrect agent role.

**Diagnosis**:
```bash
# Check agent role
mnemosyne agent info

# Check configuration
cat .mnemosyne/config.toml | grep orchestrator_bypass
```

**Solution**:
```bash
# Enable orchestrator bypass
echo '[branch_isolation]
orchestrator_bypass = true' >> .mnemosyne/config.toml

# Ensure agent has correct role
# (Orchestrator role must be set at agent creation)
```

### Issue: Conflicts not being detected

**Cause**: Conflict detection disabled or file tracker not running.

**Diagnosis**:
```bash
# Check conflict detection status
cat .mnemosyne/config.toml | grep -A5 conflict_detection

# Check file tracker logs
mnemosyne logs | grep "FileTracker"
```

**Solution**:
```bash
# Enable conflict detection
echo '[conflict_detection]
enabled = true' >> .mnemosyne/config.toml

# Restart agents
mnemosyne restart
```

### Issue: Status line not updating

**Cause**: Status command not integrated or cache issue.

**Solutions**:

1. **Test status command directly**:
   ```bash
   mnemosyne-status --format ansi
   ```

2. **Clear cache**:
   ```bash
   rm -rf .mnemosyne/cache/
   ```

3. **Verify shell integration**:
   ```bash
   # Bash
   grep mnemosyne ~/.bashrc

   # Zsh
   grep mnemosyne ~/.zshrc
   ```

4. **Re-source configuration**:
   ```bash
   source ~/.bashrc  # or ~/.zshrc
   ```

### Issue: Read-only access denied

**Cause**: Auto-approve disabled or configuration error.

**Diagnosis**:
```bash
# Check auto-approve setting
cat .mnemosyne/config.toml | grep auto_approve_readonly
```

**Solution**:
```bash
# Enable auto-approve
echo '[branch_isolation]
auto_approve_readonly = true' >> .mnemosyne/config.toml
```

**Note**: Read-only access should NEVER be denied by design. If it is, this is a bug.

### Issue: Branch registry file corrupted

**Symptoms**:
- JSON parse errors
- Invalid agent assignments
- Missing branches

**Solution**:
```bash
# Backup corrupted file
mv .mnemosyne/branch_registry.json .mnemosyne/branch_registry.json.backup

# Reset registry (loses current assignments)
rm .mnemosyne/branch_registry.json

# Restart and re-join
mnemosyne branch join <branch> <intent>
```

**Prevention**:
- Enable registry persistence
- Regular backups
- Atomic writes (enabled by default)

## Debugging Tools

### Verbose Logging
```bash
# Enable debug logging
export RUST_LOG=mnemosyne=debug

# Run command
mnemosyne branch status
```

### State Inspection
```bash
# View raw registry
cat .mnemosyne/branch_registry.json | jq

# View process registry
cat .mnemosyne/process_registry.json | jq

# View coordination queue
ls -la .mnemosyne/coordination_queue/
```

### Monitoring
```bash
# Watch for file changes
watch -n 1 'ls -la .mnemosyne/'

# Monitor heartbeats
tail -f .mnemosyne/logs/heartbeat.log

# Track conflicts
tail -f .mnemosyne/logs/conflicts.log
```

## Performance Issues

### Issue: High coordination overhead

**Symptoms**:
- Slow response times
- Frequent notifications
- High CPU usage

**Solutions**:

1. **Reduce poll interval**:
   ```toml
   [cross_process]
   poll_interval_seconds = 5  # Increase from default 2
   ```

2. **Disable features selectively**:
   ```toml
   [notifications]
   periodic_interval_minutes = 60  # Less frequent

   [conflict_detection]
   enabled = false  # For non-critical work
   ```

3. **Partition work more aggressively**:
   ```bash
   # More specific file scopes
   mnemosyne branch join main write --files src/specific_file.rs
   ```

### Issue: File tracker memory usage

**Cause**: Large number of tracked modifications.

**Solution**:
```bash
# Clear old modifications
mnemosyne gc --older-than 24h

# Reduce retention
echo '[file_tracker]
retention_hours = 12' >> .mnemosyne/config.toml
```

## Getting Help

### Check Logs
```bash
# System logs
mnemosyne logs

# Specific component
mnemosyne logs --component branch-coordinator
mnemosyne logs --component conflict-notifier
```

### Export Debug Bundle
```bash
# Create debug bundle for issue reporting
mnemosyne debug export --output debug.tar.gz

# Includes:
# - Configuration
# - Registry state
# - Recent logs
# - Process information
```

### Report Issues
When reporting issues, include:
1. Mnemosyne version: `mnemosyne --version`
2. Configuration: `.mnemosyne/config.toml`
3. Steps to reproduce
4. Expected vs actual behavior
5. Debug bundle (if possible)

## Known Limitations

1. **No merge conflict resolution**: Branch isolation prevents work conflicts, not git merge conflicts
2. **File-level granularity**: Conflicts detected at file level, not line level
3. **Eventual consistency**: Cross-process coordination has ~2 second latency
4. **Single repository**: Does not coordinate across multiple repository clones

## Emergency Procedures

### Complete Reset
```bash
# Stop all agents
mnemosyne stop-all

# Clear all state
rm -rf .mnemosyne/

# Reinitialize
mnemosyne init

# Rejoin branches
mnemosyne branch join <branch> <intent>
```

### Force Release All Assignments
```bash
# Dangerous: Releases all assignments
mnemosyne branch force-release-all

# Use only when:
# - All agents crashed
# - State is corrupted
# - Deadlock detected
```

### Manual State Repair
```bash
# Edit registry manually (last resort)
$EDITOR .mnemosyne/branch_registry.json

# Validate JSON
cat .mnemosyne/branch_registry.json | jq empty

# Restart
mnemosyne branch status
```

## Prevention Checklist

- [ ] Branch isolation enabled in config
- [ ] Auto-approve read-only enabled
- [ ] Conflict detection configured
- [ ] Notifications configured appropriately
- [ ] Cross-process coordination enabled (if using multiple processes)
- [ ] `.mnemosyne/` in `.gitignore`
- [ ] Regular backups of registry
- [ ] Monitoring/logging enabled
- [ ] Shell integration tested
- [ ] Team trained on coordination workflows
