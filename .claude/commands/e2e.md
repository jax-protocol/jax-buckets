---
description: Run end-to-end dev environment tests
allowed-tools:
  - Bash(./bin/dev)
  - Bash(./bin/dev *)
  - Bash(tmux capture-pane *)
  - Bash(tmux has-session *)
  - Bash(sleep *)
  - Read
  - Grep
  - Glob
---

Run end-to-end tests of the dev environment to verify fixtures and cross-node sync.

Read `agents/DEBUG.md` for dev environment commands and debugging.

**Expected end state is documented in `bin/dev_/fixtures.toml`** - see the "EXPECTED END STATE" comment at the end of that file for what to verify.

## E2E Test Flow

1. `./bin/dev kill --force && ./bin/dev clean` - Clean start
2. `./bin/dev run --background` - Start nodes
3. Wait for health: `./bin/dev api full health`
4. Verify fixtures: `./bin/dev api full list` and `./bin/dev api full ls <id> /`
5. Check cross-node sync: `./bin/dev api app list`
6. Check for errors: `./bin/dev logs grep ERROR`

## Report Format

```
## E2E Test Results

### Node Health
- full: [OK/FAIL]
- app: [OK/FAIL]
- gw: [OK/FAIL]

### Fixtures
- Bucket created: [yes/no]
- Files uploaded: [yes/no]
- Share (owner): [yes/no]
- Share (mirror): [yes/no]
- Publish: [yes/no]
- Move operation: [yes/no]

### Cross-Node Sync
- App sees bucket: [yes/no]
- App can read files: [yes/no]

### Errors
[List any errors from logs]

### Summary
[PASS/FAIL] - [description]
```
