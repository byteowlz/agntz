# Distribution Files

This directory contains package manager templates for distributing agntz.

## Files

### AUR (Arch User Repository)
- `PKGBUILD` - Main binary package (auto-generated during release)
- `PKGBUILD-git` - Git build version (manual install only)

### Homebrew
- `agntz.rb` - Homebrew formula template (auto-updated in byteowlz/homebrew-tap)

### Scoop
- `agntz.json` - Scoop manifest (auto-updated in byteowlz/scoop-bucket)

## Release Process

1. Tag a new release: `git tag v0.X.Y && git push --tags`
2. GitHub Actions builds binaries for all platforms
3. Actions creates GitHub release with checksums
4. Actions publishes to:
   - **AUR**: Updates PKGBUILD with new checksums
   - **Homebrew**: Triggers webhook to update formula in homebrew-tap
   - **Scoop**: Updates manifest in scoop-bucket

## Manual Installation

### AUR
```bash
paru -S agntz  # or yay/pacman
```

### Homebrew
```bash
brew install byteowlz/tap/agntz
```

### Scoop
```bash
scoop bucket add byteowlz https://github.com/byteowlz/scoop-bucket
scoop install agntz
```
