#!/usr/bin/env bash
# Setup script for initial AUR package creation
# Run this locally to prepare the AUR package submission

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== AUR Package Setup Script for agntz ===${NC}\n"

# Check if git is installed
if ! command -v git &> /dev/null; then
    echo -e "${RED}Error: git is not installed${NC}"
    exit 1
fi

# Check if makepkg is installed
if ! command -v makepkg &> /dev/null; then
    echo -e "${RED}Error: makepkg is not installed (are you on Arch Linux?)${NC}"
    exit 1
fi

# Create temporary directory
TEMP_DIR=$(mktemp -d)
echo -e "${YELLOW}Working in temporary directory: $TEMP_DIR${NC}\n"

# Copy PKGBUILD from dist
echo "Copying PKGBUILD from dist/aur/PKGBUILD..."
cp "$(dirname "$0")/../dist/aur/PKGBUILD" "$TEMP_DIR/PKGBUILD"

# Copy .SRCINFO from dist
echo "Copying .SRCINFO from dist/aur/.SRCINFO..."
cp "$(dirname "$0")/../dist/aur/.SRCINFO" "$TEMP_DIR/.SRCINFO"

# Generate .SRCINFO from PKGBUILD (recommended)
echo "Generating fresh .SRCINFO..."
cd "$TEMP_DIR"
makepkg --printsrcinfo > .SRCINFO

# Initialize git repo
echo "Initializing git repository..."
git init
git config user.name "byteowlz"
git config user.email "dev@byteowlz.com"

# Add files
git add PKGBUILD .SRCINFO

# Commit
git commit -m "Initial commit: agntz v0.3.0"

echo -e "\n${GREEN}=== Package prepared! ===${NC}"
echo -e "\nNext steps:"
echo -e "1. Test build locally: ${YELLOW}makepkg -si${NC}"
echo -e "2. Verify installation: ${YELLOW}agntz --version${NC}"
echo -e "3. If test passes, push to AUR:"
echo -e "   ${YELLOW}cd $TEMP_DIR${NC}"
echo -e "   ${YELLOW}git remote add origin ssh://aur@aur.archlinux.org/agntz.git${NC}"
echo -e "   ${YELLOW}git push -u origin main${NC}"
echo -e "\nNote: You need to have an AUR account and SSH key configured."
echo -e "See SETUP.md for details.\n"

# Ask if user wants to test build
read -p "Test build locally? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "\n${YELLOW}Testing build...${NC}"
    if makepkg -si; then
        echo -e "\n${GREEN}Build successful!${NC}"
        if command -v agntz &> /dev/null; then
            VERSION=$(agntz --version 2>&1 || echo "unknown")
            echo -e "agntz installed: $VERSION"
        fi
    else
        echo -e "\n${RED}Build failed. Please check the errors above.${NC}"
    fi
fi

echo -e "\n${GREEN}Done!${NC}"
echo -e "The package files are in: ${YELLOW}$TEMP_DIR${NC}"
echo -e "Keep this directory until you've successfully pushed to AUR.\n"
