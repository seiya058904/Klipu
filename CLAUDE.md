# Klipu

A focused Windows clipboard manager, forked from CoPanda v3.4.0 (GPL-3.0-only). Keep it small: fix and refine rather than add. `README.md` covers the architecture and `NOTICE.md` the relationship with upstream.

## Commands

```powershell
py scripts/sidecar.py --debug      # required before compiling the workspace; `python` may be a Store stub here
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
bash scripts/rules.sh
cd app; npm run lint; npm run build; npm test
cd app; npx tauri build --bundles nsis --no-sign   # after `py scripts/sidecar.py`
node scripts/third-party.mjs       # after any dependency change
```

## Rules that are easy to break

- No comments in Rust under `crates` and `app/src-tauri/src`; `scripts/rules.sh` enforces this.
- Tests go in a sibling `*_test.rs` declared last in the file, never inline.
- `unsafe` only in `crates/cp-win-sys`. `cp-core` has no platform code and no printing.
- Every string needs both languages: `say::pick(zh, en)` and the `ZH` and `EN` tables in `app/src/locales.ts`.
- Colors live in both `crates/cp-panel/ui/theme.slint` and `app/src/index.css`, plus the window background in `app/src-tauri/src/looks.rs`. Change them together.

## Do not regress

- **Coexistence with CoPanda:** never touch `CopyPaste` folders, startup entries, processes or uninstall keys. Klipu's identity is `Klipu` everywhere (data folder, `Run\Klipu`, `%TEMP%\Klipu`, the `Klipu Worn` clipboard format), and its defaults avoid CoPanda's shortcuts.
- **No network:** Klipu makes no requests of its own. Do not add updaters, telemetry or remote content.
- **Stored data:** the `Kind` enum and the schema still hold kinds the interface merged (`token`, `json`, `audio`, and others). They must keep displaying, searching and pasting. `.cpbackup` files must stay importable.
- **Chinese search:** `search::fold` splits CJK into single characters and `terms_of` rebuilds them as phrases. The tests in `search_test.rs` and `store_listing_test.rs` cover this.

## Testing the real app

Run with `LOCALAPPDATA` pointed at a scratch folder so real history stays untouched. A first run adds `HKCU\...\Run\Klipu`, so remove that value afterwards. `crates/cp-store/examples/seed.rs` fills a history for screenshots and performance checks.
