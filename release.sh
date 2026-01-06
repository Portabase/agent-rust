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

if [ -f package.json ]; then
  echo "Updating package.json to version $CLEAN_VERSION..."
  sed -i "s/\"version\": \".*\"/\"version\": \"$CLEAN_VERSION\"/" package.json
fi

if [ -f pyproject.toml ]; then
  echo "Updating pyproject.toml to version $CLEAN_VERSION..."
  sed -i "s/^version = \".*\"/version = \"$CLEAN_VERSION\"/" pyproject.toml
fi

if [ -f Cargo.toml ]; then
  echo "Updating Cargo.toml to version $CLEAN_VERSION..."
  sed -i "s/^version = \".*\"/version = \"$CLEAN_VERSION\"/" Cargo.toml
fi

echo "Updating CITATION.cff..."
sed -i "s/^version: .*/version: $CLEAN_VERSION/" CITATION.cff
sed -i "s/^date-released: .*/date-released: \"$CURRENT_DATE\"/" CITATION.cff

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