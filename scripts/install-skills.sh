#!/usr/bin/env bash
# Link every Agent Skill in this repo into the local harness directories.
#
# Usage:
#   bash scripts/install-skills.sh --dry-run
#   bash scripts/install-skills.sh
#
# Existing real directories are skipped, never deleted.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DRY_RUN=0

case "${1:-}" in
  --dry-run) DRY_RUN=1 ;;
  "") ;;
  -h|--help)
    sed -n '2,8p' "$0"
    exit 0
    ;;
  *)
    echo "usage: bash scripts/install-skills.sh [--dry-run]" >&2
    exit 2
    ;;
esac

if [ -z "${HOME:-}" ]; then
  echo "error: HOME is not set" >&2
  exit 1
fi

DESTS=("$HOME/.claude/skills" "$HOME/.agents/skills")
count=0

for dest in "${DESTS[@]}"; do
  if [ "$DRY_RUN" -eq 0 ]; then
    mkdir -p "$dest"
  fi

  while IFS= read -r -d '' skill_md; do
    src="$(dirname "$skill_md")"
    name="$(basename "$src")"
    target="$dest/$name"

    if [ -e "$target" ] && [ ! -L "$target" ]; then
      printf 'skip  %s (exists, not a symlink)\n' "$target"
      continue
    fi

    if [ "$DRY_RUN" -eq 1 ]; then
      printf 'would link %s -> %s\n' "$target" "$src"
    else
      ln -sfn "$src" "$target"
      printf 'linked %s -> %s\n' "$target" "$src"
    fi
    count=$((count + 1))
  done < <(find "$REPO/skills" -name SKILL.md -type f -not -path '*/node_modules/*' -print0)
done

if [ "$count" -eq 0 ]; then
  echo "error: no skills found under $REPO/skills" >&2
  exit 1
fi

printf '\n%d skill link(s) %s.\n' "$count" "$([ "$DRY_RUN" -eq 1 ] && echo planned || echo installed)"
