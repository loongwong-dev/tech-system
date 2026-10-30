#!/usr/bin/env bash
# AgentDev Charter — language-agnostic repository quality gates.
#
# 仅依赖 bash、git 与 POSIX 常用文本工具（grep、awk、sed、find、sort、head、tr 等）。
# 用法：
#   bash scripts/check.sh all        # docs + format + optional markdownlint + commits + secrets
#   bash scripts/check.sh docs       # 文档结构与站内链接
#   bash scripts/check.sh format     # 换行、行尾空白、Tab、末尾换行
#   bash scripts/check.sh commits    # Conventional Commits（支持 CI 范围）
#   bash scripts/check.sh secrets    # 轻量密钥特征扫描（gitleaks 的补充，不替代）
#   bash scripts/check.sh links      # 站内链接；设置 CHECK_EXTERNAL_LINKS=1 时调用 lychee

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

fail=0
errors=0

err() {
  printf 'ERROR: %s\n' "$*" >&2
  fail=1
  errors=$((errors + 1))
}

info() {
  printf '\n==> %s\n' "$*"
}

list_files() {
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git ls-files --cached --others --exclude-standard
  else
    find . -type f ! -path './.git/*' ! -path '*/node_modules/*' ! -path '*/vendor/*' | sed 's|^\./||'
  fi
}

is_binary_path() {
  case "$1" in
    *.png|*.jpg|*.jpeg|*.gif|*.webp|*.ico|*.pdf|*.zip|*.gz|*.tgz|*.tar|*.woff|*.woff2|*.ttf|*.otf|*.mp4|*.mov|*.mp3)
      return 0
      ;;
  esac
  return 1
}

is_text_path() {
  case "$1" in
    *.md|*.yml|*.yaml|*.json|*.jsonc|*.toml|*.sh|*.bash|*.txt|*.ini|*.cfg|*.conf)
      return 0
      ;;
    .editorconfig|.gitignore|.gitattributes|LICENSE|NOTICE|Makefile|makefile|GNUmakefile)
      return 0
      ;;
  esac
  return 1
}

check_h1() {
  local f="$1" first count
  first="$(awk 'NF { print; exit }' "$f")"
  case "$first" in
    '# '*) ;;
    *) err "$f: first non-empty line must be an H1 ('# ...')" ;;
  esac
  count="$(awk '
    /^[[:space:]]*(```|~~~)/ { fence = !fence; next }
    !fence && /^# / { n++ }
    END { print n + 0 }
  ' "$f")"
  [ "$count" -eq 1 ] || err "$f: expected exactly one H1, found $count"
}

check_docs() {
  info "Required documents"
  local required
  for required in \
    README.md \
    AGENTS.md \
    CONTRIBUTING.md \
    CHANGELOG.md \
    SECURITY.md \
    docs/README.md \
    docs/governance.md \
    docs/ai-behavior.md \
    docs/ci-cd.md \
    docs/configuration.md \
    docs/adr/README.md \
    docs/adr/template.md; do
    [ -f "$required" ] || err "missing required document: $required"
  done

  info "Document headings"
  local f base
  for f in README.md AGENTS.md CONTRIBUTING.md CHANGELOG.md SECURITY.md; do
    [ -f "$f" ] && check_h1 "$f"
  done
  while IFS= read -r f; do
    [ -f "$f" ] || continue
    check_h1 "$f"
    base="${f##*/}"
    case "$base" in
      README.md) ;;
      *) printf '%s\n' "$base" | grep -qE '^[a-z0-9]+(-[a-z0-9]+)*\.md$' \
        || err "$f: filename must be lowercase kebab-case" ;;
    esac
  done < <(find docs -name '*.md' -type f 2>/dev/null | sort)

  info "Documentation index coverage"
  if [ -f docs/README.md ]; then
    local index_links
    index_links="$(extract_links docs/README.md)"
    while IFS= read -r f; do
      [ "$f" = "docs/README.md" ] && continue
      local rel="${f#docs/}"
      if ! printf '%s\n' "$index_links" | grep -Fxq "$rel"; then
        err "$f: not linked from docs/README.md as ($rel)"
      fi
    done < <(find docs -name '*.md' -type f 2>/dev/null | sort)
  fi

  info "ADR naming and index"
  local adr_index=""
  [ -f docs/adr/README.md ] && adr_index="$(extract_links docs/adr/README.md)"
  while IFS= read -r f; do
    base="${f##*/}"
    case "$base" in
      README.md|template.md) continue ;;
    esac
    if ! printf '%s\n' "$base" | grep -qE '^[0-9]{4}-[a-z0-9-]+\.md$'; then
      err "$f: ADR filename must match NNNN-lowercase-kebab.md"
    fi
    if ! printf '%s\n' "$adr_index" | grep -Fxq "$base"; then
      err "$f: not indexed in docs/adr/README.md as ($base)"
    fi
  done < <(find docs/adr -maxdepth 1 -name '*.md' -type f 2>/dev/null | sort)

  check_local_links
}

extract_links() {
  # 跳过围栏代码块，避免把示例里的 Markdown 链接当成真实链接。
  # 同时提取内联链接与引用式定义 [ref]: path。
  awk '
    /^[[:space:]]*(```|~~~)/ { fence = !fence; next }
    !fence { print }
  ' "$1" | awk '
    {
      line = $0
      while (match(line, /\]\([^)]+\)/)) {
        print substr(line, RSTART + 2, RLENGTH - 3)
        line = substr(line, RSTART + RLENGTH)
      }
      if (match($0, /^[[:space:]]*\[[^]]+\]:[[:space:]]*[^[:space:]]+/)) {
        ref = substr($0, RSTART, RLENGTH)
        sub(/^.*\]:[[:space:]]*/, "", ref)
        print ref
      }
    }
  ' 2>/dev/null || true
}

check_local_links() {
  info "Local Markdown links"
  local f link path
  while IFS= read -r f; do
    [ -f "$f" ] || continue
    is_binary_path "$f" && continue
    while IFS= read -r link; do
      [ -n "$link" ] || continue
      case "$link" in
        *'{{'*|*'<'*|*'>'*) continue ;;
      esac
      case "$link" in
        http://*|https://*|mailto:*|tel:*|ftp://*|\#*) continue ;;
      esac
      [ "${link#\#}" != "$link" ] && continue
      link="${link%%#*}"
      link="${link%%\?*}"
      [ -n "$link" ] || continue
      link="${link%\"}"
      link="${link%\'}"
      link="${link%% \"*}"
      link="${link//%20/ }"
      case "$link" in
        /*) path="${link#/}" ;;
        *) path="$(dirname "$f")/$link" ;;
      esac
      if [ ! -e "$path" ]; then
        err "$f: broken local link: $link"
      fi
    done < <(extract_links "$f")
  done < <(list_files | grep -E '\.md$' | sort)
}

check_format() {
  info "Text formatting"
  local f
  while IFS= read -r f; do
    [ -f "$f" ] || continue
    is_binary_path "$f" && continue
    is_text_path "$f" || continue

    if grep -q $'\r' "$f"; then
      err "$f: CRLF line ending found (expected LF; *.bat/*.cmd excluded)"
    fi

    if [ -s "$f" ] && [ -n "$(tail -c 1 "$f")" ]; then
      err "$f: missing final newline"
    fi

    case "$f" in
      *.md|*.yml|*.yaml|*.json|*.jsonc|*.toml|*.sh|*.bash)
        if grep -q $'\t' "$f"; then
          err "$f: tab character found; use spaces"
        fi
        ;;
    esac

    case "$f" in
      *.md) ;; # 两个行尾空格是合法硬换行，交给 markdownlint MD009。
      *)
        if grep -qE '[[:space:]]+$' "$f"; then
          err "$f: trailing whitespace found"
        fi
        ;;
    esac
  done < <(list_files)
}

check_commit_subject() {
  local subject="$1"
  local re='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9._/-]+\))?!?: .+'
  local emoji="" rest="$subject" type=""
  case "$subject" in
    'Merge '*|'Revert "'*|'fixup! '*|'squash! '*) return 0 ;;
  esac
  case "$subject" in
    *' '*)
      emoji="${subject%% *}"
      rest="${subject#* }"
      ;;
  esac
  case "$emoji" in
    "✨"|"🐛"|"📝"|"💄"|"♻️"|"⚡"|"⚡️"|"✅"|"📦"|"📦️"|"👷"|"🔧"|"⏪"|"⏪️"|"💥") ;;
    *) emoji=""; rest="$subject" ;;
  esac
  if [ -n "$emoji" ]; then
    if [ "$emoji" = "💥" ]; then
      if ! printf '%s\n' "$rest" | grep -qE '^[a-z]+(\([a-z0-9._/-]+\))?!:'; then
        err "commit emoji '💥' requires a breaking type suffix, e.g. '💥 feat!: ...'"
      fi
    else
      type="$(printf '%s\n' "$rest" | sed -n 's/^\([a-z]*\).*/\1/p')"
      case "$type:$emoji" in
        feat:✨|fix:🐛|docs:📝|style:💄|refactor:♻️|perf:⚡|perf:⚡️|test:✅|build:📦|build:📦️|ci:👷|chore:🔧|revert:⏪|revert:⏪️) ;;
        *) err "commit emoji '$emoji' does not match type '$type' (see CONTRIBUTING.md)" ;;
      esac
    fi
    subject="$rest"
  fi
  if ! printf '%s\n' "$subject" | grep -qE "$re"; then
    err "commit message is not Conventional Commits: $subject (scope must match [a-z0-9._/-]; types: see CONTRIBUTING.md)"
    return 0
  fi
  if [ "${#subject}" -gt 100 ]; then
    err "commit subject exceeds 100 characters (${#subject}): $subject"
  fi
  case "$subject" in
    *.) err "commit subject must not end with a period: $subject" ;;
  esac
}

check_commits() {
  info "Commit messages"
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    err "not a git repository; cannot check commit messages"
    return 0
  fi

  local range="" subjects="" base="${BASE_SHA:-}" head="${HEAD_SHA:-}"
  if [ -n "$base" ] && [ -n "$head" ] && [ "$base" != "0000000000000000000000000000000000000000" ] \
    && git rev-parse --verify --quiet "$base^{commit}" >/dev/null \
    && git rev-parse --verify --quiet "$head^{commit}" >/dev/null; then
    range="$base..$head"
  fi

  if [ -n "$range" ]; then
    subjects="$(git log --format='%s' "$range" 2>/dev/null || true)"
  else
    subjects="$(git log -1 --format='%s' 2>/dev/null || true)"
  fi

  if [ -z "$subjects" ]; then
    info "no commits to check"
    return 0
  fi

  local subject
  while IFS= read -r subject; do
    [ -n "$subject" ] && check_commit_subject "$subject"
  done <<< "$subjects"
}

check_secrets() {
  info "Secret pattern scan (baseline)"
  local patterns=(
    'AKIA[0-9A-Z]{16}'
    'ASIA[0-9A-Z]{16}'
    'gh[pousr]_[A-Za-z0-9]{36,}'
    'github_pat_[A-Za-z0-9_]{22,}'
    'glpat-[A-Za-z0-9_-]{20,}'
    'xox[baprs]-[A-Za-z0-9-]{10,}'
    'AIza[0-9A-Za-z_-]{35}'
    'npm_[A-Za-z0-9]{36,}'
    '-----BEGIN (RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----'
  )
  local f p hits line location rest
  while IFS= read -r f; do
    [ -f "$f" ] || continue
    is_binary_path "$f" && continue
    for p in "${patterns[@]}"; do
      hits="$(grep -nHE -- "$p" "$f" 2>/dev/null || true)"
      [ -n "$hits" ] || continue
      while IFS= read -r line; do
        case "$line" in
          *secret-scan:allow*) ;;
          *)
            location="${line%%:*}"
            rest="${line#*:}"
            err "possible secret ($p): ${location}:${rest%%:*} (value redacted)"
            ;;
        esac
      done <<< "$hits"
    done
  done < <(list_files)
}

check_external_links() {
  [ "${CHECK_EXTERNAL_LINKS:-0}" = "1" ] || return 0
  info "External links"
  if command -v lychee >/dev/null 2>&1; then
    lychee --config .lychee.toml --no-progress './**/*.md' || err "lychee reported broken external links"
  else
    err "CHECK_EXTERNAL_LINKS=1 but lychee is not installed"
  fi
}

check_markdown() {
  info "Markdown lint (optional)"
  if command -v markdownlint-cli2 >/dev/null 2>&1; then
    markdownlint-cli2 "**/*.md" || err "markdownlint-cli2 reported issues"
  else
    info "markdownlint-cli2 not installed; skipped"
  fi
}

usage() {
  cat <<'EOF'
Usage: bash scripts/check.sh [all|docs|format|commits|secrets|links]

  all      docs + format + optional markdownlint + commits + secrets
  docs     required docs, H1, index coverage, ADR naming, local links
  format   CRLF, final newline, tabs, trailing whitespace
  commits  Conventional Commits (uses BASE_SHA/HEAD_SHA)
  secrets  baseline secret pattern scan
  links    docs checks plus optional lychee (CHECK_EXTERNAL_LINKS=1)
EOF
}

case "${1:-all}" in
  all)
    check_docs
    check_format
    check_markdown
    check_commits
    check_secrets
    ;;
  docs)
    check_docs
    ;;
  format)
    check_format
    ;;
  commits)
    check_commits
    ;;
  secrets)
    check_secrets
    ;;
  links)
    check_docs
    check_external_links
    ;;
  -h|--help|help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

if [ "$fail" -ne 0 ]; then
  printf '\n%d check(s) failed.\n' "$errors" >&2
  exit 1
fi

printf '\nAll checks passed.\n'
