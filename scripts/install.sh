#!/bin/sh

set -eu

REPO_OWNER="mitosis-org"
REPO_NAME="mito-reth"
BIN_NAME="mito-reth"

usage() {
  cat <<EOF
Install ${BIN_NAME} from GitHub Releases.

Usage:
  sh install.sh [--version <tag>] [--install-dir <dir>]

Options:
  --version <tag>      Install a specific release tag (default: latest)
  --install-dir <dir>  Install directory (default: /usr/local/bin or ~/.local/bin)
  -h, --help           Show this help
EOF
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

detect_platform() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux) platform_os="linux" ;;
    Darwin) platform_os="macos" ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT) platform_os="windows" ;;
    *)
      echo "error: unsupported operating system: $os" >&2
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64) platform_arch="amd64" ;;
    aarch64|arm64) platform_arch="arm64" ;;
    *)
      echo "error: unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  if [ "$platform_os" = "windows" ] && [ "$platform_arch" = "arm64" ]; then
    echo "error: Windows arm64 release artifact is not published" >&2
    exit 1
  fi
}

resolve_install_dir() {
  if [ -n "${INSTALL_DIR:-}" ]; then
    return
  fi

  if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
  else
    INSTALL_DIR="${HOME}/.local/bin"
  fi
}

build_asset_name() {
  asset_base="${BIN_NAME}-${platform_os}-${platform_arch}"
  if [ "$platform_os" = "windows" ]; then
    ASSET_NAME="${asset_base}.zip"
    EXTRACTED_BIN="${BIN_NAME}.exe"
    INSTALLED_BIN="${BIN_NAME}.exe"
  else
    ASSET_NAME="${asset_base}.tar.gz"
    EXTRACTED_BIN="${BIN_NAME}"
    INSTALLED_BIN="${BIN_NAME}"
  fi
}

build_download_url() {
  if [ -n "${VERSION:-}" ]; then
    DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${ASSET_NAME}"
  else
    DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/latest/download/${ASSET_NAME}"
  fi
}

download_asset() {
  archive_path="${TMP_DIR}/${ASSET_NAME}"
  echo "Downloading ${DOWNLOAD_URL}" >&2
  curl --fail --location --silent --show-error "${DOWNLOAD_URL}" --output "${archive_path}"
}

extract_asset() {
  extract_dir="${TMP_DIR}/extract"
  mkdir -p "${extract_dir}"

  case "${ASSET_NAME}" in
    *.tar.gz)
      need_cmd tar
      tar -xzf "${archive_path}" -C "${extract_dir}"
      ;;
    *.zip)
      if command -v unzip >/dev/null 2>&1; then
        unzip -q "${archive_path}" -d "${extract_dir}"
      elif command -v powershell.exe >/dev/null 2>&1; then
        powershell.exe -NoProfile -Command "Expand-Archive -Path '$(cygpath -w "${archive_path}" 2>/dev/null || printf "%s" "${archive_path}")' -DestinationPath '$(cygpath -w "${extract_dir}" 2>/dev/null || printf "%s" "${extract_dir}")' -Force" >/dev/null
      else
        echo "error: unzip or powershell.exe is required to extract ${ASSET_NAME}" >&2
        exit 1
      fi
      ;;
  esac

  BIN_SOURCE="$(find "${extract_dir}" -type f -name "${EXTRACTED_BIN}" | head -n 1)"
  if [ -z "${BIN_SOURCE}" ]; then
    echo "error: failed to locate extracted binary ${EXTRACTED_BIN}" >&2
    exit 1
  fi
}

install_binary() {
  mkdir -p "${INSTALL_DIR}"
  install_path="${INSTALL_DIR}/${INSTALLED_BIN}"

  if command -v install >/dev/null 2>&1; then
    install -m 0755 "${BIN_SOURCE}" "${install_path}"
  else
    cp "${BIN_SOURCE}" "${install_path}"
    chmod 0755 "${install_path}"
  fi

  echo "Installed ${INSTALLED_BIN} to ${install_path}" >&2
}

VERSION=""
INSTALL_DIR=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="${2:-}"
      if [ -z "${VERSION}" ]; then
        echo "error: --version requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:-}"
      if [ -z "${INSTALL_DIR}" ]; then
        echo "error: --install-dir requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

need_cmd curl
need_cmd uname
need_cmd mktemp
need_cmd find
need_cmd head

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

detect_platform
resolve_install_dir
build_asset_name
build_download_url
download_asset
extract_asset
install_binary
