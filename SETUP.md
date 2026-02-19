# Release Setup

This document outlines the setup required for automated releases to AUR, Homebrew, and Scoop.

## Prerequisites

### GitHub Secrets

The following secrets must be configured in the repository:

1. **TAP_GITHUB_TOKEN** - Personal access token with `repo` scope for updating Homebrew tap
2. **AUR_EMAIL** - Email address for AUR commits
3. **AUR_SSH_PRIVATE_KEY** - SSH private key for AUR package upload
4. **SCOOP_GITHUB_TOKEN** - Personal access token with `repo` scope for updating Scoop bucket

### AUR Setup

1. Create an AUR account at https://aur.archlinux.org/
2. Generate SSH key pair for AUR:
   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/aur -C "AUR SSH Key"
   ```
3. Add the public key to your AUR account settings
4. Create the `agntz` package in AUR by copying the initial PKGBUILD from `dist/aur/PKGBUILD`
5. Add `AUR_SSH_PRIVATE_KEY` secret to GitHub repo with the contents of the private key
6. Add `AUR_EMAIL` secret with your email address

**Note:** The AUR package only needs to be created once. Subsequent updates are automated.

### Homebrew Tap

The Homebrew tap repository already exists at https://github.com/byteowlz/homebrew-tap with an automated workflow that generates formulas dynamically from release data.

**Action required:** Just ensure the `TAP_GITHUB_TOKEN` secret is configured in the agntz repository.

The workflow automatically:
- Downloads checksums from the release
- Fetches the repo description from GitHub API
- Generates the formula with correct URLs and checksums
- Commits and pushes the update

### Scoop Bucket

The Scoop bucket repository already exists at https://github.com/byteowlz/scoop-bucket with an automated workflow that generates manifests dynamically from release data.

**Action required:** Just ensure the `SCOOP_GITHUB_TOKEN` secret is configured in the agntz repository.

The workflow automatically:
- Downloads checksums from the release
- Fetches the repo description from GitHub API
- Generates the manifest with correct URLs and checksums
- Commits and pushes the update

## Release Process

Once everything is set up, releases are automated:

### First Release

For the very first release:
1. Manually create the AUR package using the PKGBUILD from `dist/aur/PKGBUILD`
2. Verify Homebrew and Scoop secrets are set
3. Run `just release-bump 0.3.1` to bump version and push tag
4. Verify the workflows create the initial formula/manifest files

### Subsequent Releases

For subsequent releases, everything is automated:

1. Run `just release-bump 0.4.0` to bump version and push tag
2. GitHub Actions automatically:
   - Builds binaries for all platforms (Linux, macOS, Windows)
   - Creates GitHub release with checksums
   - Updates AUR package with new checksums
   - Updates Homebrew formula with new checksums
   - Updates Scoop manifest with new checksums

3. Users can then install via:
   ```bash
   # AUR
   paru -S agntz

   # Homebrew
   brew install byteowlz/tap/agntz

   # Scoop
   scoop bucket add byteowlz https://github.com/byteowlz/scoop-bucket
   scoop install agntz
   ```

## Verification

After each release, verify:

1. [ ] GitHub release contains all artifacts
2. [ ] AUR package builds successfully
3. [ ] Homebrew formula installs correctly
4. [ ] Scoop manifest installs correctly
5. [ ] Checksums match downloaded files
