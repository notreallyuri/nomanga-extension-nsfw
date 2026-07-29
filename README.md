# nomanga-extension-nsfw

Adult source pack for [nomanga](https://github.com/notreallyuri/nomanga).
Bundles four sources into one sandboxed WebAssembly plugin.

**Extension id:** `dev.yuri.nsfwpack` · **ABI:** 5

Every source here is flagged `nsfw`, so the app hides them until adult sources
are enabled.

## Installing

In nomanga, open Settings → Extensions and add this repository:

```
https://notreallyuri.github.io/nomanga-extension-nsfw/index.min.json
```

This pack lives in its own repository on purpose: nothing here is visible to
anyone who has not added that URL. The pack then shows up under *Available*,
with the domains it may reach listed before you confirm.

## Sources

| Source | Id | Language |
|---|---|---|
| nHentai (V2) | `net.nhentai.v2` | multi |
| Hitomi.la | `la.hitomi` | multi |
| MadaraDex | `org.madaradex` | en |
| E-Hentai | `org.ehentai` | multi |

Each declares its own network allow-list; the pack as a whole may reach
`nhentai.net`, `api.nhentai.net`, `hitomi.la`, `*.hitomi.la`,
`ltn.gold-usergeneratedcontent.net`, `madaradex.org`, `cdn.madaradex.org`,
`e-hentai.org`, `api.e-hentai.org`, `ehgt.org`, and `*.hath.network`. The host
enforces this — a source cannot reach anything it did not declare.

## Settings

All optional; every source works unconfigured.

**nHentai**

| Setting | Type | Purpose |
|---|---|---|
| Preferred Language | select | Filters homepage and searches |
| API Key | secret | For V2 endpoints |
| Global Included Tags | text | Added to every search |
| Global Excluded Tags | text | Removed from every search |

**Hitomi.la** — Preferred Language (select).

**E-Hentai** — accepts pasted forum cookies rather than an API key. Log in at
`forums.e-hentai.org`, then copy two cookie values:

| Setting | Cookie | Effect |
|---|---|---|
| Member ID | `ipb_member_id` | Raises the image limit, unlocks the multi-page viewer |
| Pass Hash | `ipb_pass_hash` | Paired with the above |

**MadaraDex** — no settings. Its CDN is shielded, and the pack handles that
internally.

## Building

```sh
cargo build --release
# → target/wasm32-unknown-unknown/release/extension_nsfw.wasm
```

`.cargo/config.toml` pins `wasm32-unknown-unknown`, so no `--target` is needed.
If the toolchain is missing it: `rustup target add wasm32-unknown-unknown`.

Install the resulting `.wasm` through Settings → Extensions → *Install from
file…*, or inspect it without the app using the CLI in the main repo:

```sh
cargo run -p nomanga-cli -- --wasm path/to/extension_nsfw.wasm info
cargo run -p nomanga-cli -- --wasm path/to/extension_nsfw.wasm --source la.hitomi homepage
```

## Relationship to nomanga

This repo depends on `nomanga-sdk` from the main repository:

```toml
nomanga-sdk = { git = "https://github.com/notreallyuri/nomanga", branch = "main" }
```

`Cargo.lock` pins the resolved commit, so the SDK only moves when you ask:

```sh
cargo update -p nomanga-sdk
```

The host checks `abi_version` on load and refuses anything outside the range it
supports. If the app reports an ABI error after an update, rebuild the pack
against the current SDK.
