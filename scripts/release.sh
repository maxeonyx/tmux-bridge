#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

if [[ -n "$(git status --porcelain)" ]]; then
  printf 'Error: release requires a clean git worktree.\n' >&2
  exit 1
fi

current_version=$(grep -m1 '^version = "' Cargo.toml | cut -d '"' -f 2)

if [[ -n "${1-}" ]]; then
  version=$1
else
  IFS=. read -r major minor patch <<< "$current_version"
  version="$major.$minor.$((patch + 1))"
fi

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'Error: version must look like 0.1.5\n' >&2
  exit 1
fi

tag="v$version"
install_root="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}"
install_path="$install_root/bin/tb"

if [[ $version == "$current_version" ]]; then
  printf 'Error: version %s is already current in Cargo.toml\n' "$version" >&2
  exit 1
fi

if git rev-parse "$tag" >/dev/null 2>&1; then
  printf 'Error: git tag %s already exists\n' "$tag" >&2
  exit 1
fi

printf 'Running test ratchet...\n'
python3 scripts/ratchet.py

printf 'Updating Cargo.toml to %s...\n' "$version"
sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"$/s//version = \"$version\"/" Cargo.toml

printf 'Building project...\n'
cargo build

printf 'Committing version bump...\n'
git add Cargo.toml Cargo.lock
git commit -m "Bump version to $tag"

printf 'Tagging %s...\n' "$tag"
git tag "$tag"

printf 'Pushing commit...\n'
git push

printf 'Pushing tags...\n'
git push --tags

printf 'Installing locally...\n'
cargo install --path .

printf '\nReleased %s\n' "$tag"
printf 'Installed binary: %s\n' "$install_path"
