# Update Summary: Sync Checking & Integration Tests

## What Was Added

### 1. Updated AGENTS.md
- Added missing schedule commands: `show`, `status`, `next`
- Updated command documentation
- Added new "Keeping Commands in Sync" section with usage instructions

### 2. Integration Test Suite (`tests/integration/`)

Created 5 test scripts that verify agntz correctly wraps each tool:

| Script | Tool | Tests |
|--------|------|-------|
| `test_skdlr.sh` | skdlr | add, list, show, edit, remove, enable/disable, run, logs, status, next, backend |
| `test_mmry.sh` | mmry | add, search, list, export (json/md), import, stats, stores, remove |
| `test_trx.sh` | trx | create, list, show, update, close, ready commands |
| `test_mailz.sh` | mailz-cli | inbox, reserve, release, reservations |
| `test_hstry.sh` | hstry | search with all flag combinations |

**Runner script:** `run_all.sh` - executes all tests and reports summary

### 3. Sync Check Script (`scripts/check_sync.sh`)

Automated tool that compares agntz's commands against wrapped tools:

**Features:**
- Detects new flags in wrapped tools that agntz doesn't support
- Identifies flag naming mismatches (e.g., `--enabled` vs `--disabled`)
- Finds new commands that agntz could expose
- Reports versions of all wrapped tools
- Provides actionable suggestions for fixing issues

**Current Issues Detected:**
- ⚠️ skdlr has `--at` flag for one-off runs (not in agntz)
- ⚠️ skdlr uses `--enabled` but agntz uses `--disabled` flag
- ℹ️ Various optional commands in mmry/trx that could be exposed

## Usage

### Run Integration Tests
```bash
# All tests
./tests/integration/run_all.sh

# Single tool
./tests/integration/test_mmry.sh
```

### Check Sync Status
```bash
./scripts/check_sync.sh
```

### Check Tool Health
```bash
agntz tools doctor
```

## Directory Structure

```
agntz/
├── AGENTS.md                          # Updated with new section
├── scripts/
│   ├── check_sync.sh                  # NEW: Sync checking tool
│   └── README.md                      # NEW: Scripts documentation
└── tests/
    └── integration/
        ├── README.md                  # NEW: Test documentation
        ├── run_all.sh                 # NEW: Test runner
        ├── test_skdlr.sh              # NEW: skdlr tests
        ├── test_mmry.sh               # NEW: mmry tests
        ├── test_trx.sh                # NEW: trx tests
        ├── test_mailz.sh              # NEW: mailz tests
        └── test_hstry.sh              # NEW: hstry tests
```

## Recommendations

### Immediate Actions

1. **Fix skdlr sync issues** in `src/schedule.rs`:
   - Add `--at` flag for one-off runs
   - Change `--disabled` to `--enabled` flag
   - Update help text to mention natural language schedules

2. **Add to CI/CD**:
   ```yaml
   - name: Check tool sync
     run: ./scripts/check_sync.sh
   - name: Run integration tests
     run: ./tests/integration/run_all.sh
   ```

3. **Update AGENTS.md** after fixing issues to reflect new `--at` flag and `--enabled`

### Long-term Considerations

1. **Schedule regular sync checks**: Weekly or on each release
2. **Pin wrapped tool versions**: Add `Cargo.toml` dependencies or documentation
3. **Add version matrix**: Document which agntz version works with which tool versions
4. **Consider feature flags**: Allow users to opt-in to new tool features

## Testing Performed

- ✅ All integration scripts are executable
- ✅ `test_mmry.sh` runs successfully (10/10 tests pass)
- ✅ `check_sync.sh` detects known issues
- ✅ Documentation updated and verified

## Next Steps

1. Update `src/schedule.rs` to fix detected skdlr issues
2. Run full integration test suite after fixes
3. Consider adding more optional commands (trx `dep`, mmry `ingest`, etc.)
4. Set up CI/CD automation for continuous checking
