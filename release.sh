#!/bin/bash

set -e

if [ -z "$1" ]; then
  echo "Usage: ./release <version>"
  echo "Example: ./release v1.0.0"
  exit 1
fi

VERSION=$1
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

if [ "$CURRENT_BRANCH" = "main" ]; then
  if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: On 'main' branch, only release tags (X.Y.Z) are allowed."
    exit 1
  fi
else
  if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+-rc\.[0-9]+$ ]]; then
    echo "Error: On branch '$CURRENT_BRANCH', only RC tags matching X.Y.Z-rc.W are allowed (e.g., 1.0.0-rc.1)."
    exit 1
  fi
fi

CLEAN_VERSION=${VERSION#v}
CURRENT_DATE=$(date +%Y-%m-%d)

echo "Preparing release $VERSION..."

# Detect platform for sed
if sed --version >/dev/null 2>&1; then
  # GNU/Linux
  SED_INPLACE="sed -i"
else
  # macOS/BSD
  SED_INPLACE="sed -i ''"
fi

# Update package.json
[ -f package.json ] && echo "Updating package.json to version $CLEAN_VERSION..." && \
  $SED_INPLACE "s/\"version\": \".*\"/\"version\": \"$CLEAN_VERSION\"/" package.json

# Update pyproject.toml
[ -f pyproject.toml ] && echo "Updating pyproject.toml to version $CLEAN_VERSION..." && \
  $SED_INPLACE "s/^version = \".*\"/version = \"$CLEAN_VERSION\"/" pyproject.toml

# Update Cargo.toml
[ -f Cargo.toml ] && echo "Updating Cargo.toml to version $CLEAN_VERSION..." && \
  $SED_INPLACE "s/^version = \".*\"/version = \"$CLEAN_VERSION\"/" Cargo.toml

# Update CITATION.cff only if it exists
if [ -f CITATION.cff ]; then
  echo "Updating CITATION.cff..."
  $SED_INPLACE "s/^version: .*/version: $CLEAN_VERSION/" CITATION.cff
  $SED_INPLACE "s/^date-released: .*/date-released: \"$CURRENT_DATE\"/" CITATION.cff
fi


git add .

if ! git diff-index --quiet HEAD --; then
  echo "Committing changes..."
  git commit -m "chore(release): $VERSION"
else
  echo "No changes to commit. Proceeding to tag..."
fi

if git rev-parse "$VERSION" >/dev/null 2>&1; then
    echo "Tag $VERSION already exists. Aborting."
    exit 1
fi

echo "Creating tag $VERSION..."
git tag -a "$VERSION" -m "Release $VERSION"

echo "Pushing changes and tags to remote..."
git push
git push origin "$VERSION"

echo "Successfully released $VERSION!"