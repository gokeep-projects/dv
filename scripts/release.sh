#!/bin/bash
set -e

BUMP_TYPE=${1:-patch}
CURRENT_BRANCH=$(git branch --show-current)

echo "=== DevTool Release ==="
echo "Branch: $CURRENT_BRANCH"

LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
CURRENT_VERSION=${LATEST_TAG#v}
echo "Current: $CURRENT_VERSION"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

case $BUMP_TYPE in
  major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
  minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
  patch) PATCH=$((PATCH + 1)) ;;
  *) echo "Usage: $0 [major|minor|patch]"; exit 1 ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
NEW_TAG="v${NEW_VERSION}"
echo "New: $NEW_VERSION ($NEW_TAG)"

# Generate changelog
if [ "$LATEST_TAG" != "v0.0.0" ]; then
  CHANGELOG=$(git log ${LATEST_TAG}..HEAD --pretty=format:"- %s (%h)" --no-merges 2>/dev/null || echo "- Initial release")
else
  CHANGELOG=$(git log --pretty=format:"- %s (%h)" --no-merges -20 2>/dev/null || echo "- Initial release")
fi

echo ""
echo "Changes:"
echo "$CHANGELOG"

# Commit uncommitted changes
if ! git diff --quiet HEAD 2>/dev/null; then
  echo ""
  echo "Committing changes..."
  git add -A
  git commit -m "release: $NEW_TAG

$CHANGELOG"
fi

# Create tag
git tag -a "$NEW_TAG" -m "Release $NEW_VERSION

$CHANGELOG"

echo ""
echo "✅ Tag $NEW_TAG created"
echo ""
echo "Push with: git push origin $NEW_TAG"
echo "GitHub Actions will build and release automatically."
