# Contributing

Klipu aims to do less, and do it reliably. Bug fixes are welcome. Please open an issue before starting a new feature, since the answer may be no.

Contributions are licensed under [GPL-3.0-only](LICENSE). There is no contributor agreement.

## Setup

Follow **Build from source** in the [README](README.md). The app embeds the panel as a sidecar, so build it once before compiling the workspace:

```powershell
py scripts/sidecar.py --debug
```

## Checks

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
bash scripts/rules.sh
cd app; npm run lint; npm run build; npm test
```

If a change adds or removes a dependency, regenerate the notices with `node scripts/third-party.mjs`. The app shows these files under **About**.

## Conventions

- **No comments in Rust.** `scripts/rules.sh` rejects `//` and `/*` under `crates` and `app/src-tauri/src`. Choose a name that explains the code instead.
- **Tests live in a sibling file**, named after the file with a `_test.rs` suffix and declared last:

  ```rust
  #[cfg(test)]
  #[path = "watch_test.rs"]
  mod tests;
  ```

- **`unsafe` stays in `crates/cp-win-sys`.**
- **`cp-core` stays platform-free** and does not print.
- **Every user-facing string exists in both languages:** `say::pick("中文", "English")` in Rust, and the `ZH` and `EN` tables in `app/src/locales.ts`.
- **Respect the data.** Stored items must stay readable after a change, including kinds the interface no longer shows separately.
- **Keep the panel and the settings window in step.** Colors are defined in `crates/cp-panel/ui/theme.slint` and `app/src/index.css`, and should match.

Commit messages follow `type(scope): the concrete change`, in English.
