# Release Setup Summary

This document summarizes the release automation setup completed for agntz.

## What Was Set Up

### 1. Distribution Templates (`dist/`)

#### AUR (Arch User Repository)
- `dist/aur/PKGBUILD` - Main binary package template
- `dist/aur/PKGBUILD-git` - Git build version for manual install
- `dist/aur/.SRCINFO` - AUR metadata file

#### Homebrew
- `dist/homebrew/agntz.rb` - Homebrew formula template (reference)
- **Note:** The actual formula is generated dynamically by the homebrew-tap workflow

#### Scoop
- `dist/scoop/agntz.json` - Scoop manifest template (reference)
- **Note:** The actual manifest is generated dynamically by the scoop-bucket workflow

#### Documentation
- `dist/README.md` - Overview of distribution files
- `dist/RELEASE.md` - Quick reference for release process
- `dist/VERIFICATION.md` - Post-release verification checklist

### 2. Release Configuration

- `release.toml` - cargo-release configuration for automated version bumping

### 3. GitHub Workflow

- `.github/workflows/release.yml` - Enhanced with:
  - Windows builds (x64, ARM64) for Scoop
  - Scoop publishing step
  - Multi-format artifact handling (tar.gz and zip)

### 4. Justfile Commands

- `just release-bump <version>` - Bump version, commit, tag, and push
- `just release-check` - Verify release readiness
- `just release <type>` - Use cargo-release for version management

### 5. Documentation

- `SETUP.md` - Complete setup guide for all package managers
- `README.md` - Updated with installation instructions for all package managers

## Package Managers

### AUR (Arch Linux)
- **Status:** Ready (requires manual AUR package creation)
- **Automated:** Yes (via GitHub Actions)
- **Installation:** `paru -S agntz`

### Homebrew (macOS/Linux)
- **Status:** Ready (homebrew-tap repo and workflow exist)
- **Automated:** Yes (via webhook to byteowlz/homebrew-tap)
- **Installation:** `brew install byteowlz/tap/agntz`

### Scoop (Windows)
- **Status:** Ready (scoop-bucket repo and workflow exist)
- **Automated:** Yes (via webhook to byteowlz/scoop-bucket)
- **Installation:** `scoop bucket add byteowlz https://github.com/byteowlz/scoop-bucket && scoop install agntz`

## Supported Platforms

| Platform | Architecture | Package Format |
|----------|-------------|----------------|
| Linux    | x86_64      | tar.gz         |
| Linux    | aarch64     | tar.gz         |
| macOS    | x86_64      | tar.gz         |
| macOS    | aarch64     | tar.gz         |
| Windows  | x86_64      | zip            |
| Windows  | aarch64     | zip            |

## Release Workflow

```
Tag pushed → Build all platforms → Create release → Publish to:
  ├── AUR (update PKGBUILD with checksums)
  ├── Homebrew (trigger webhook to create formula)
  └── Scoop (trigger webhook to create manifest)
```

## Required GitHub Secrets

These secrets must be configured in the agntz repository:

1. `TAP_GITHUB_TOKEN` - For Homebrew tap updates
2. `AUR_EMAIL` - Email for AUR commits
3. `AUR_SSH_PRIVATE_KEY` - SSH key for AUR package management
4. `SCOOP_GITHUB_TOKEN` - For Scoop bucket updates

## First-Time Setup

### AUR (One-time manual setup)

1. Create AUR account at https://aur.archlinux.org/
2. Generate SSH key pair
3. Upload public key to AUR account
4. Manually create the agntz package using the PKGBUILD from `dist/aur/PKGBUILD`
5. Configure GitHub secrets

### Homebrew & Scoop

No manual setup needed - the repositories and workflows already exist. Just configure the GitHub secrets.

## Making a Release

```bash
# Check everything is ready
just release-check

# Bump version and create release
just release-bump 0.3.1

# The workflow automatically handles the rest
```

## Verification

After each release, use the checklist in `dist/VERIFICATION.md` to verify:
- All artifacts are present on GitHub
- Checksums are correct
- Package managers update successfully
- All platforms install and run correctly

## Files Changed

### Created
- `dist/aur/PKGBUILD`
- `dist/aur/PKGBUILD-git`
- `dist/aur/.SRCINFO`
- `dist/homebrew/agntz.rb`
- `dist/scoop/agntz.json`
- `dist/README.md`
- `dist/RELEASE.md`
- `dist/VERIFICATION.md`
- `release.toml`
- `SETUP.md`
- `RELEASE_SETUP_SUMMARY.md` (this file)

### Modified
- `.github/workflows/release.yml` - Added Windows builds and Scoop publishing
- `justfile` - Added release commands
- `README.md` - Added package manager installation instructions

## Next Steps

1. Configure required GitHub secrets
2. Create AUR package (one-time setup)
3. Run first release to verify everything works
4. Use `VERIFICATION.md` checklist to test all platforms
5. Document any issues and adjust as needed
