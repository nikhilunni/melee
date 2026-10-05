#!/usr/bin/env bash
# Build the web app into target/web and serve it (WebGPU browsers only).
#   tools/run-web.sh            build and serve on http://localhost:8080
#   tools/run-web.sh --build    build only
#   PORT=9000 tools/run-web.sh
#   tools/run-web.sh --disc harness/roms/GALE01.iso
#       development: also serve the image read-only at /disc.iso; open
#       http://localhost:8080/?disc=disc.iso&autostart=Fox:Marth:FinalDestination
# The wasm-bindgen CLI must match the wasm-bindgen crate exactly; the
# matching version is installed under target/tools, never ~/.cargo/bin.
set -euo pipefail
build_only=0
disc=()
while [[ $# -gt 0 ]]; do
    case $1 in
        --build) build_only=1 ;;
        --disc) disc=(--disc "$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"); shift ;;
        *) echo "unknown argument $1" >&2; exit 1 ;;
    esac
    shift
done
cd "$(dirname "$0")/.."
root="$PWD"
out="$root/target/web"

rustup target list --installed | grep -qx wasm32-unknown-unknown \
    || { echo 'missing target: rustup target add wasm32-unknown-unknown' >&2; exit 1; }

version=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/[^0-9.]/, ""); print; exit }' Cargo.lock)
bindgen="$root/target/tools/bin/wasm-bindgen"
if [[ ! -x $bindgen ]] || [[ $("$bindgen" --version) != "wasm-bindgen $version" ]]; then
    echo "installing wasm-bindgen-cli $version into target/tools" >&2
    cargo install --quiet --locked --root "$root/target/tools" wasm-bindgen-cli --version "$version"
fi

# .cargo/config.toml carries the wasm32 rustflags.
cargo build --release --target wasm32-unknown-unknown -p melee-web
rm -rf "$out"
mkdir -p "$out/pkg"
"$bindgen" --target web --no-typescript --out-dir "$out/pkg" \
    "$root/target/wasm32-unknown-unknown/release/melee_web.wasm"
if command -v wasm-opt >/dev/null; then
    wasm-opt -O2 --enable-bulk-memory --enable-nontrapping-float-to-int \
        "$out/pkg/melee_web_bg.wasm" -o "$out/pkg/melee_web_bg.wasm"
fi
cp crates/melee-web/www/* "$out/"
# The menu fonts (SIL OFL 1.1), with their licence.
mkdir -p "$out/fonts"
cp assets/fonts/*.ttf assets/fonts/OFL.txt "$out/fonts/"
printf 'built %s (%s)\n' "$out" "$(du -sh "$out/pkg/melee_web_bg.wasm" | cut -f1)"

[[ $build_only == 1 ]] && exit 0
echo "open it in Chrome or Edge (WebGPU); Ctrl-C stops"
exec python3 tools/web-serve.py "$out" --port "${PORT:-8080}" ${disc[@]+"${disc[@]}"}
