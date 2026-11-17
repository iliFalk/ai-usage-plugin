#!/usr/bin/env bash
#
# Everything CI proves on a Linux machine, in one command.
#
# Why this exists: the GitHub Actions matrix bills by operating system, and one full run of
# `.github/workflows/ci.yml` costs about 230 quota minutes — half of it the aarch64-darwin nix leg at
# the macOS rate. The gates themselves are cheap; running them on GitHub every time is not.
#
# What this script cannot cover, and says so instead of pretending: the macOS job (Keychain, WKWebView
# tray, the Swift menu bar), the Windows job (the process and TCP-table walk, the tray exe guard), and
# the aarch64 nix legs. Those need their own operating system. Everything else runs here.
#
#   ./scripts/ci-local.sh              # the whole gate
#   CI_LOCAL_SKIP_NIX=1 ./scripts/ci-local.sh   # skip the nix leg (it is the slow one)
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
pass=0
skip=0

ok()   { echo "OK   $*"; pass=$((pass + 1)); }
bad()  { echo "FAIL $*"; fail=$((fail + 1)); }
miss() { echo "SKIP $*"; skip=$((skip + 1)); }

# The flake system name for this machine, so the nix step says which system it checked.
nix_system() {
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)              echo "x86_64-linux" ;;
    Linux-aarch64|Linux-arm64) echo "aarch64-linux" ;;
    Darwin-arm64)              echo "aarch64-darwin" ;;
    Darwin-x86_64)             echo "x86_64-darwin" ;;
    *)                         echo "unknown" ;;
  esac
}

step() {
  local name="$1"
  shift
  echo "== $name"
  if "$@"; then ok "$name"; else bad "$name"; fi
}

# --- the Linux job of ci.yml --------------------------------------------------
step "changelog sections and versions" make changelog-check
step "format"                          cargo fmt --all -- --check
step "clippy (-D warnings)"            cargo clippy --all-targets --locked -- -D warnings
step "tests (all targets)"             cargo test --all-targets --locked
step "release build"                   cargo build --release --locked

# The declared minimum, which the msrv jobs check on three systems. A dependency bump can raise the
# floor without touching a line of this crate, and that only shows up on an older rustc.
msrv="$(sed -n 's/^rust-version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)"
if [ -n "$msrv" ] && rustup toolchain list 2>/dev/null | grep -q "^$msrv"; then
  step "declared minimum rustc $msrv" cargo "+$msrv" check --all-targets --locked
elif [ -n "$msrv" ]; then
  miss "declared minimum rustc $msrv — not installed (rustup toolchain install $msrv)"
else
  miss "declared minimum rustc — Cargo.toml declares no rust-version"
fi
step "desktop models (node)"           make desktop-test
step "popover SSR contract (vite)"     make popover-ssr-test
step "omarchy plugin model"            make plugin-test
step "linux mint tray model"           make mint-test

# The runtime test drives GTK through PyGObject. CI installs it with apt; here it is either present
# or the step is named as a gap.
# `import gi` alone is not enough: the module can be installed while the Gtk 3 typelib is not, and the
# test then fails on `gi.require_version('Gtk', '3.0')` instead of being reported as a gap.
if /usr/bin/python3 -c "import gi; gi.require_version('Gtk', '3.0')" 2>/dev/null; then
  step "linux mint GTK runtime" make mint-runtime-test
else
  miss "linux mint GTK runtime — PyGObject is not installed for /usr/bin/python3"
fi

if command -v cargo-machete >/dev/null 2>&1; then
  step "unused dependencies" cargo machete
else
  miss "unused dependencies — cargo-machete is not installed (cargo install cargo-machete)"
fi

# --- the nix leg, for the system this machine is ---------------------------------
# The flake is checked in the project's own container so the host keeps no nix install. A named volume
# keeps the store, so a second run does not rebuild the world.
if [ "${CI_LOCAL_SKIP_NIX:-0}" = "1" ]; then
  miss "nix flake check — skipped by CI_LOCAL_SKIP_NIX"
elif ! command -v docker >/dev/null 2>&1; then
  miss "nix flake check — docker is not installed"
elif ! docker info >/dev/null 2>&1; then
  miss "nix flake check — the docker daemon is not reachable"
else
  echo "== nix flake check ($(nix_system))"
  if docker run --rm -v "$PWD":/src -v ai-usagebar-nix-store:/nix -w /src \
       -e NIX_CONFIG="experimental-features = nix-command flakes" nixos/nix \
       sh -c 'git config --global --add safe.directory /src; nix flake check --print-build-logs' \
       > /tmp/ci-local-nix.log 2>&1; then
    ok "nix flake check — log in /tmp/ci-local-nix.log"
  else
    bad "nix flake check — log in /tmp/ci-local-nix.log"
  fi
fi

# The nix job also proves the built program runs, not just that it compiles.
if [ -x target/release/ai-usagebar ]; then
  if ./target/release/ai-usagebar --version >/dev/null 2>&1; then
    ok "release binary --version"
  else
    bad "release binary --version"
  fi
else
  miss "release binary --version — no target/release/ai-usagebar"
fi

echo
echo "== $pass passed, $fail failed, $skip skipped"
if [ "$fail" -ne 0 ]; then
  echo "   the gate is red: do not merge on this run"
  exit 1
fi
if [ "$skip" -ne 0 ]; then
  echo "   green for what ran here; the skipped steps are named above"
fi
echo "   not covered by this script at all: test (macos-latest), test (windows-latest), nix aarch64 legs"
exit 0
