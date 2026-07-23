#!/bin/bash
# Version bump script for Arca
# Usage: ./scripts/bump_version.sh [patch|minor|major]

set -e

PART="${1:-patch}"
VERSION_FILE="crates/arca-cli/src/main.rs"
INSTALL_SCRIPT="scripts/install.sh"

# Get current version
get_version() {
    grep 'ARCA_VERSION' "$VERSION_FILE" | sed 's/.*"\([0-9]*\.[0-9]*\.[0-9]*\).*/\1/' | head -1
}

bump_version() {
    local current="$1"
    local part="$2"
    
    IFS='.' read -r major minor patch <<< "$current"
    
    case $part in
        patch)
            patch=$((patch + 1))
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
    esac
    
    echo "${major}.${minor}.${patch}"
}

CURRENT=$(get_version)
NEW_VERSION=$(bump_version "$CURRENT" "$PART")

echo "Current version: $CURRENT"
echo "New version: $NEW_VERSION"

# Update crates/arca-cli/src/main.rs
sed -i '' "s/ARCA_VERSION.*\".*\"/ARCA_VERSION = \"${NEW_VERSION}-alpha\"/" "$VERSION_FILE"

# Update scripts/install.sh
sed -i '' "s/ARCA_VERSION=\".*\"/ARCA_VERSION=\"${NEW_VERSION}\"/" "$INSTALL_SCRIPT"

echo "✅ Version bumped to $NEW_VERSION"
echo ""
echo "Next steps:"
echo "  make build      # Build with new version"
echo "  make install    # Install new version"
echo "  git add -A && git commit -m 'Bump version to $NEW_VERSION'"
