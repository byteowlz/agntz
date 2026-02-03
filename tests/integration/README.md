# Integration Tests

This directory contains integration tests for agntz and the tools it wraps.

## Running Tests

Run all integration tests:
```bash
./tests/integration/run_all.sh
```

Run a specific tool's tests:
```bash
./tests/integration/test_skdlr.sh
./tests/integration/test_mmry.sh
./tests/integration/test_trx.sh
./tests/integration/test_mailz.sh
./tests/integration/test_hstry.sh
```

## Sync Check

Check if agntz commands are in sync with the wrapped CLI tools:

```bash
./scripts/check_sync.sh
```

This script will:
- Compare command flags between agntz and the wrapped tools
- Detect new commands or flags that agntz doesn't expose
- Report flag naming mismatches
- Suggest which modules need updating

## Test Coverage

Each test script checks:
- **test_skdlr.sh**: Schedule commands (add, list, show, edit, run, logs, enable/disable)
- **test_mmry.sh**: Memory commands (add, search, list, export, import, stats, stores)
- **test_trx.sh**: Task/issue commands (create, list, show, update, close)
- **test_mailz.sh**: Mail and reservation commands (inbox, reserve, release)
- **test_hstry.sh**: Search commands (search with various flags)

## CI Integration

To add these tests to CI, add to `.github/workflows/test.yml`:

```yaml
name: Integration Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install tools
        run: |
          cargo install mmry-cli skdlr trx mailz hstry-cli
      - name: Build agntz
        run: cargo build
      - name: Run sync check
        run: ./scripts/check_sync.sh
      - name: Run integration tests
        run: ./tests/integration/run_all.sh
```

## Fixing Sync Issues

When the sync check reports issues:

1. **New flags**: Add the new flag to the appropriate module in `src/`
2. **Flag naming changes**: Update flag names to match the wrapped tool
3. **New commands**: Add new subcommands to the enum and handler functions

Example for skdlr `--at` flag:

```rust
// In src/schedule.rs
pub enum ScheduleCommand {
    Add {
        name: String,
        schedule: Option<String>,  // Make optional if --at is mutually exclusive
        #[arg(long)]
        at: Option<String>,        // Add --at flag
        command: String,
        // ... other fields
    },
    // ...
}
```
