# Release Verification Checklist

This checklist should be used after each release to verify that all package managers are working correctly.

## GitHub Release

- [ ] Release page shows correct version tag
- [ ] All artifacts are present:
  - [ ] `agntz-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
  - [ ] `agntz-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz`
  - [ ] `agntz-vX.Y.Z-x86_64-apple-darwin.tar.gz`
  - [ ] `agntz-vX.Y.Z-aarch64-apple-darwin.tar.gz`
  - [ ] `agntz-vX.Y.Z-x86_64-pc-windows-msvc.zip`
  - [ ] `agntz-vX.Y.Z-aarch64-pc-windows-msvc.zip`
  - [ ] `checksums.txt`
- [ ] Release notes are generated correctly
- [ ] Checksums in `checksums.txt` match downloaded files

## AUR Package

- [ ] PKGBUILD was updated in AUR
- [ ] Version number matches release
- [ ] SHA256 checksums match those in release
- [ ] Test build succeeds:
  ```bash
  git clone https://aur.archlinux.org/agntz.git
  cd agntz
  makepkg -si
  ```
- [ ] Binary installs to `/usr/bin/agntz`
- [ ] Running `agntz --version` shows correct version

## Homebrew Formula

- [ ] Formula was created in `byteowlz/homebrew-tap`
- [ ] Version number matches release
- [ ] All platform URLs are correct
- [ ] SHA256 checksums match those in release
- [ ] Test install succeeds:
  ```bash
  brew install byteowlz/tap/agntz
  ```
- [ ] Running `agntz --version` shows correct version
- [ ] `brew test agntz` passes

## Scoop Manifest

- [ ] Manifest was created in `byteowlz/scoop-bucket`
- [ ] Version number matches release
- [ ] All Windows platform URLs are correct
- [ ] SHA256 hashes match those in release
- [ ] Test install succeeds:
  ```bash
  scoop bucket add byteowlz https://github.com/byteowlz/scoop-bucket
  scoop install agntz
  ```
- [ ] Running `agntz --version` shows correct version

## Platform-Specific Testing

### Linux (x86_64)
```bash
wget https://github.com/byteowlz/agntz/releases/download/vX.Y.Z/agntz-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
tar xzf agntz-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
./agntz --version
```

### Linux (aarch64)
```bash
wget https://github.com/byteowlz/agntz/releases/download/vX.Y.Z/agntz-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz
tar xzf agntz-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz
./agntz --version
```

### macOS (Intel)
```bash
curl -LO https://github.com/byteowlz/agntz/releases/download/vX.Y.Z/agntz-vX.Y.Z-x86_64-apple-darwin.tar.gz
tar xzf agntz-vX.Y.Z-x86_64-apple-darwin.tar.gz
./agntz --version
```

### macOS (Apple Silicon)
```bash
curl -LO https://github.com/byteowlz/agntz/releases/download/vX.Y.Z/agntz-vX.Y.Z-aarch64-apple-darwin.tar.gz
tar xzf agntz-vX.Y.Z-aarch64-apple-darwin.tar.gz
./agntz --version
```

### Windows (x64)
```powershell
# Download from release page
# Extract zip
# Run: .\agntz.exe --version
```

### Windows (ARM64)
```powershell
# Download from release page
# Extract zip
# Run: .\agntz.exe --version
```

## Functionality Tests

- [ ] `agntz --version` works
- [ ] `agntz memory add "test" -c test` works
- [ ] `agntz memory search "test"` works
- [ ] `agntz tasks` works
- [ ] `agntz search "test"` works
- [ ] `agntz tools list` works
- [ ] `agntz schedule list` works
