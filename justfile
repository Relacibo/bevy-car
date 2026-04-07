# Justfile for bevy-car release management

# Get current version from Cargo.toml
current-version:
    @grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'

# Increment patch version (e.g., 0.1.0 -> 0.1.1)
_increment-patch:
    #!/usr/bin/env bash
    current=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    IFS='.' read -ra VERSION <<< "$current"
    major="${VERSION[0]}"
    minor="${VERSION[1]}"
    patch="${VERSION[2]}"
    new_patch=$((patch + 1))
    echo "$major.$minor.$new_patch"

# Create a release tag with optional version (defaults to patch increment)
release version="":
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Determine version
    if [ -z "{{version}}" ]; then
        new_version=$(just _increment-patch)
        echo "No version specified, incrementing patch: $new_version"
    else
        new_version="{{version}}"
        echo "Using specified version: $new_version"
    fi
    
    # Update Cargo.toml
    sed -i "s/^version = \".*\"/version = \"$new_version\"/" Cargo.toml
    echo "Updated Cargo.toml to version $new_version"
    
    # Update Cargo.lock
    cargo check --quiet
    
    # Git commit and tag
    git add Cargo.toml Cargo.lock
    git commit -m "chore: bump version to $new_version"
    git tag -a "v$new_version" -m "Release version $new_version"
    
    echo "✅ Created release v$new_version"

# Create release and push tag and changes to remote
release-push version="":
    just release {{version}}
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Get current version from Cargo.toml
    current=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    tag_version="v$current"
    
    # Get current branch
    current_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Push branch and tag
    git push origin "$current_branch"
    git push origin "$tag_version"
    
    echo "✅ Pushed $tag_version and $current_branch to remote"
    echo "GitHub Pages deployment will start automatically"

# Build WASM for release
build-wasm:
    ./scripts/build-release-wasm.sh

# List all tags
list-tags:
    @git tag -l "v*" | sort -V
