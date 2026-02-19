# Release Process Quick Reference

## Quick Steps

```bash
# 1. Check release readiness
just release-check

# 2. Bump version and create release tag
just release-bump 0.3.1

# 3. Monitor the release workflow
# Go to: https://github.com/byteowlz/agntz/actions/workflows/release.yml
```

## What Happens Automatically

The release workflow runs through these steps:

1. **Build Phase**
   - Builds binaries for 6 platforms
   - Packages them as tar.gz (Unix) or zip (Windows)
   - Uploads artifacts

2. **Release Phase**
   - Downloads all artifacts
   - Generates SHA256 checksums
   - Creates GitHub release with all files

3. **Publish Phase**
   - Updates AUR PKGBUILD with new checksums
   - Triggers Homebrew tap workflow to create formula
   - Triggers Scoop bucket workflow to create manifest

## Manual AUR Setup (First Time Only)

If this is the first release, manually create the AUR package:

```bash
# Clone the template PKGBUILD
cat > PKGBUILD << 'EOF'
pkgname=agntz
pkgver=0.3.0
pkgrel=1
pkgdesc='Agent utility toolkit for AI coding agents'
arch=('x86_64' 'aarch64')
url='https://github.com/byteowlz/agntz'
license=('MIT')
depends=('gcc-libs')
source_x86_64=("$pkgname-$pkgver.tar.gz::https://github.com/byteowlz/agntz/releases/download/v$pkgver/agntz-v$pkgver-x86_64-unknown-linux-gnu.tar.gz")
source_aarch64=("$pkgname-$pkgver.tar.gz::https://github.com/byteowlz/agntz/releases/download/v$pkgver/agntz-v$pkgver-aarch64-unknown-linux-gnu.tar.gz")
sha256sums_x86_64=('SKIP')
sha256sums_aarch64=('SKIP')

package() {
    install -Dm755 agntz "$pkgdir/usr/bin/agntz"
}
EOF

# Submit to AUR
mkdir agntz
cd agntz
# Copy PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git init
git add PKGBUILD .SRCINFO
git commit -m "Initial commit"
git remote add origin ssh://aur@aur.archlinux.org/agntz.git
git push -u origin main
```

## Troubleshooting

### AUR Update Fails

Check:
- SSH key is configured correctly in secrets
- AUR email is set correctly
- Package name matches (should be `agntz`)

### Homebrew Update Fails

Check:
- `TAP_GITHUB_TOKEN` has `repo` scope
- Token is not expired
- Workflow exists in homebrew-tap repo

### Scoop Update Fails

Check:
- `SCOOP_GITHUB_TOKEN` has `repo` scope
- Token is not expired
- Workflow exists in scoop-bucket repo

### Windows Build Fails

The Windows builds use the default `windows-latest` runner. If builds fail:
- Check Rust version compatibility
- Verify cross-compilation isn't needed (native builds)
- Review build logs in Actions

## Verification

After release, use `VERIFICATION.md` checklist to verify all platforms work correctly.
