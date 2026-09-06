#!/usr/bin/env bash
# Install sat-hx-ide from GitHub release binaries.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/vlevasseur073/sat-helix-ide/main/install.sh | bash
#
# Optional environment variables:
#   VERSION      Release version without leading v (default: latest)
#   INSTALL_DIR  Install directory (default: ~/.local/bin)
#   REPO         GitHub repo (default: vlevasseur073/sat-helix-ide)
set -euo pipefail

REPO="${REPO:-vlevasseur073/sat-helix-ide}"
BIN="${BIN:-sat-hx-ide}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)
      case "$arch" in
        x86_64 | amd64) echo "x86_64-unknown-linux-gnu" ;;
        *)
          echo "error: unsupported Linux architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64) echo "x86_64-apple-darwin" ;;
        arm64 | aarch64) echo "aarch64-apple-darwin" ;;
        *)
          echo "error: unsupported macOS architecture: $arch" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "error: unsupported OS: $os" >&2
      echo "Windows: download the .zip from https://github.com/${REPO}/releases" >&2
      exit 1
      ;;
  esac
}

latest_version() {
  local tag
  tag="$(
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
      | head -n 1
  )"
  if [ -z "$tag" ]; then
    echo "error: could not resolve latest release from GitHub" >&2
    exit 1
  fi
  echo "${tag#v}"
}

download() {
  local url="$1"
  local dest="$2"
  curl -fsSL --proto '=https' --tlsv1.2 -o "$dest" "$url"
}

main() {
  need_cmd curl
  need_cmd tar
  need_cmd uname
  need_cmd mktemp

  local version target archive url tmpdir
  version="${VERSION:-$(latest_version)}"
  version="${version#v}"
  target="$(detect_target)"
  archive="${BIN}-${version}-${target}.tar.gz"
  url="https://github.com/${REPO}/releases/download/v${version}/${archive}"

  echo "Installing ${BIN} ${version} (${target})"
  echo "  from ${url}"
  echo "  into ${INSTALL_DIR}"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  download "$url" "${tmpdir}/${archive}"
  tar -xzf "${tmpdir}/${archive}" -C "$tmpdir"

  if [ ! -f "${tmpdir}/${BIN}" ]; then
    echo "error: archive did not contain ${BIN}" >&2
    exit 1
  fi

  mkdir -p "$INSTALL_DIR"
  install -m 755 "${tmpdir}/${BIN}" "${INSTALL_DIR}/${BIN}"

  echo "Installed ${INSTALL_DIR}/${BIN}"
  if ! command -v "$BIN" >/dev/null 2>&1; then
    echo "note: ${INSTALL_DIR} is not on PATH; add it or move the binary"
  else
    "${INSTALL_DIR}/${BIN}" version || true
  fi
}

main "$@"
