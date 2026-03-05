#!/usr/bin/env bash
# Plue CLI installer — https://plue.dev
#
# Install latest:
#   curl -fsSL https://plue.dev/install.sh | bash
#
# Install specific version:
#   PLUE_VERSION=v0.1.0 curl -fsSL https://plue.dev/install.sh | bash
#
# Install to custom directory:
#   PLUE_INSTALL_DIR=~/.local/bin curl -fsSL https://plue.dev/install.sh | bash
#
set -euo pipefail

# GitHub repo for release downloads (public mirror of plue.dev/Plue-AI/cli)
GITHUB_REPO="Plue-AI/cli"
BINARY="plue"
INSTALL_DIR="${PLUE_INSTALL_DIR:-/usr/local/bin}"

# ── Detect OS ────────────────────────────────────────────────────
detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "darwin" ;;
    *)       echo "unsupported" ;;
  esac
}

# ── Detect architecture ─────────────────────────────────────────
detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  echo "amd64" ;;
    aarch64|arm64) echo "arm64" ;;
    *)             echo "unsupported" ;;
  esac
}

# ── Resolve latest version tag ───────────────────────────────────
latest_version() {
  local url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"

  if command -v curl > /dev/null 2>&1; then
    curl -fsSL "$url" | grep '"tag_name"' | head -1 | cut -d'"' -f4
  elif command -v wget > /dev/null 2>&1; then
    wget -qO- "$url" | grep '"tag_name"' | head -1 | cut -d'"' -f4
  else
    echo "Error: curl or wget is required" >&2
    exit 1
  fi
}

# ── Download helper ──────────────────────────────────────────────
download() {
  local url="$1" dest="$2"
  if command -v curl > /dev/null 2>&1; then
    curl -fsSL -o "$dest" "$url"
  elif command -v wget > /dev/null 2>&1; then
    wget -qO "$dest" "$url"
  fi
}

# ── Main ─────────────────────────────────────────────────────────
main() {
  local os arch version archive url tmpdir

  os="$(detect_os)"
  arch="$(detect_arch)"

  if [ "$os" = "unsupported" ]; then
    echo "Error: unsupported operating system: $(uname -s)" >&2
    echo "Plue supports Linux and macOS." >&2
    exit 1
  fi
  if [ "$arch" = "unsupported" ]; then
    echo "Error: unsupported architecture: $(uname -m)" >&2
    echo "Plue supports x86_64 (amd64) and aarch64 (arm64)." >&2
    exit 1
  fi

  version="${PLUE_VERSION:-$(latest_version)}"
  if [ -z "$version" ]; then
    echo "Error: could not determine latest version." >&2
    echo "Set PLUE_VERSION=v0.1.0 to install a specific version." >&2
    exit 1
  fi

  archive="${BINARY}-${version}-${os}-${arch}.tar.gz"
  url="https://github.com/${GITHUB_REPO}/releases/download/${version}/${archive}"

  echo "Installing ${BINARY} ${version} (${os}/${arch})..."

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  echo "Downloading ${url}..."
  download "$url" "${tmpdir}/${archive}"

  echo "Extracting..."
  tar xzf "${tmpdir}/${archive}" -C "$tmpdir"

  echo "Installing to ${INSTALL_DIR}/${BINARY}..."
  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
  else
    echo "(requires sudo)"
    sudo mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
  fi
  chmod +x "${INSTALL_DIR}/${BINARY}"

  echo ""
  echo "Plue CLI ${version} installed to ${INSTALL_DIR}/${BINARY}"
  echo ""
  echo "Get started:"
  echo "  plue auth login"
  echo "  plue repo list"
  echo ""

  # Verify
  if command -v plue > /dev/null 2>&1; then
    echo "Installed: $(plue --version 2>/dev/null || echo 'OK')"
  else
    echo "Note: ${INSTALL_DIR} may not be in your PATH."
    echo "Add it with: export PATH=\"${INSTALL_DIR}:\$PATH\""
  fi
}

main "$@"
