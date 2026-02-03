# Utility Scripts

This directory contains helper scripts for maintaining agntz.

## check_sync.sh

Checks if agntz commands are in sync with the wrapped CLI tools.

### Usage

```bash
./scripts/check_sync.sh
```

### What it does

- Compares command flags between agntz and skdlr, mmry, trx, mailz, hstry
- Detects new commands or flags that agntz doesn't expose
- Reports flag naming mismatches (e.g., `--enabled` vs `--disabled`)
- Suggests which source files need updating

### Example Output

```
=== Checking skdlr (schedule) ===
skdlr version: skdlr 0.2.1
Checking 'add' command flags...
✗ ISSUE: skdlr has --at flag but agntz doesn't
  skdlr now supports one-off runs with --at, but agntz hasn't been updated

⚠ WARNING: skdlr supports natural language schedules
  agntz help text may not mention this (e.g., 'every day at 9am')

✗ ISSUE: Flag mismatch: skdlr uses --enabled, agntz uses --disabled
  Update src/schedule.rs to match skdlr's flag naming
```

### When to Run

- Before releasing a new version
- After updating wrapped tool dependencies
- As part of CI/CD pipeline
- When users report command issues

## Adding New Scripts

When adding new scripts:

1. Make the script executable: `chmod +x scripts/your_script.sh`
2. Add a brief description to this README
3. Consider adding to `AGENTS.md` if it's useful for agents
4. Test on both Linux and macOS if possible

## CI Integration

Add to GitHub Actions `.github/workflows/test.yml`:

```yaml
- name: Check tool sync
  run: ./scripts/check_sync.sh
```
