#!/usr/bin/env bash
# Builds the pack and writes docs/ — the directory GitHub Pages serves. Commit
# and push the result; the index and the .wasm sit side by side, so the index's
# relative download_url resolves against whatever URL it was fetched from.
set -euo pipefail

cd "$(dirname "$0")"

NAME="nomanga nsfw pack"
DESCRIPTION="Adult source pack for nomanga. Every source here is flagged nsfw."
WEBSITE="https://github.com/notreallyuri/nomanga-extension-nsfw"
WASM="target/wasm32-unknown-unknown/release/extension_nsfw.wasm"

if ! command -v nomanga-cli >/dev/null; then
	echo "nomanga-cli not found. Install it with:" >&2
	echo "  cargo install --git https://github.com/notreallyuri/nomanga nomanga-cli" >&2
	exit 1
fi

# An older nomanga-cli earlier in PATH than the one just installed is the
# failure mode here, and 'unrecognized subcommand' does not point at it.
if ! nomanga-cli index --help >/dev/null 2>&1; then
	echo "$(command -v nomanga-cli) is too old — it has no 'index' command." >&2
	echo "Reinstall it, and check for another copy earlier in PATH:" >&2
	echo "  cargo install --force --git https://github.com/notreallyuri/nomanga nomanga-cli" >&2
	exit 1
fi

cargo build --release

rm -rf docs
mkdir -p docs
cp "$WASM" docs/

# Pages runs Jekyll over the directory otherwise, which is only ever a way for
# a file to go missing here.
touch docs/.nojekyll

nomanga-cli index \
	--name "$NAME" \
	--description "$DESCRIPTION" \
	--website "$WEBSITE" \
	--out docs/index.min.json \
	--json \
	docs/*.wasm

nomanga-cli index --name "$NAME" --out docs/index.json docs/*.wasm

echo
echo "docs/ is ready:"
ls -la docs
echo
echo "Commit and push, then serve it from Settings -> Pages"
echo "(Deploy from a branch: main, folder /docs)."
