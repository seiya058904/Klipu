<div align="center">
  <img src="resources/brand/klipu.svg" width="112" height="112" alt="Klipu"/>
  <h1>Klipu</h1>
  <p><strong>A focused clipboard manager for Windows.</strong><br/>
  No account. No telemetry. No network. Your history stays on your machine.</p>
  <p>
    <a href="https://github.com/seiya058904/Klipu/releases/latest"><img src="https://img.shields.io/github/v/release/seiya058904/Klipu?style=flat-square&label=release" alt="Latest release"/></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-4338CA?style=flat-square" alt="Windows 10 and 11"/>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-lightgrey?style=flat-square" alt="GPL-3.0"/></a>
  </p>
  <p>
    <img src="resources/screens/panel-dark.png" width="360" alt="The Klipu panel in dark mode"/>
    &nbsp;
    <img src="resources/screens/search-light.png" width="360" alt="Searching Chinese text in the Klipu panel in light mode"/>
  </p>
</div>

## What it does

Press **Ctrl+Alt+K** anywhere. The panel opens over whatever you were doing, you type a few letters, press Enter, and what you picked lands where you were typing.

- **History** of text, images, files, folders and links
- **Fast search** with substring matching for Chinese, Japanese kana and mixed text, accent-insensitive search for Latin text, and search inside screenshots through Windows' built-in OCR
- **Plain text paste** with Shift+Enter, or from anywhere with Ctrl+Alt+Shift+K
- **Pin** what you reuse, **name** items, and **drag** images and files into other apps
- **Duplicates** collapse into one item that rises to the top
- **Tray icon, start at login, automatic cleanup** by age or size, and **backup and restore**
- **Respects sensitive copies:** content an app marks as excluded from clipboard history, as most password managers do, is never read
- **Simplified Chinese and English,** light and dark themes

## How it relates to CoPanda

Klipu is an independent, modified version of [CoPanda](https://github.com/rgdevment/CopyPaste) by Mario Hidalgo G. (rgdevment), distributed under the same GPL-3.0 license. It is not affiliated with or endorsed by the original author. See [NOTICE.md](NOTICE.md).

Klipu takes CoPanda's mature clipboard engine and removes what most people rarely need:

| | CoPanda | Klipu |
| :-- | :-- | :-- |
| Platforms | Windows, macOS | Windows |
| Languages | Spanish, English | Simplified Chinese, English |
| Paste formats | Plain text and "Paste as…" conversions | Original or plain text |
| Filters | Up to 15 kinds, multiple views per kind | Text, Image, File, Link |
| Audio and video | Waveforms and rich media cards | File cards, with a system thumbnail when one exists |
| Updates | Checks and installs online | None: Klipu makes no network requests of its own |
| Chinese search | Whole runs of characters only | Any part of a sentence |

The two can be installed side by side. Klipu has its own data folder, startup entry, installer identity and default shortcuts, and it never reads or changes CoPanda's data. It can import a CoPanda `.cpbackup` file if you choose one.

## Install

Download `Klipu_<version>_x64-setup.exe` from [Releases](https://github.com/seiya058904/Klipu/releases/latest). It installs for the current user and needs no administrator rights.

The installer is not code-signed, so Windows SmartScreen may warn you. Choose **More info → Run anyway**, and compare the file with `SHA256SUMS` from the same release if you want to verify it:

```powershell
Get-FileHash .\Klipu_1.0.0_x64-setup.exe -Algorithm SHA256
```

Klipu needs Windows 10 version 1809 or later and the WebView2 runtime, which the installer sets up if it is missing. There are no automatic updates: install a newer release over the current one and your history stays.

## Shortcuts

| Where | Keys | Action |
| :-- | :-- | :-- |
| Anywhere | Ctrl+Alt+K | Open the panel |
| Anywhere | Ctrl+Alt+Shift+K | Paste the latest item as plain text |
| Panel | Enter / Shift+Enter | Paste / paste as plain text |
| Panel | ↑ ↓ | Move through the list |
| Panel | Tab / Shift+Tab | Step through the filters |
| Panel | `#image` `#图片` `#file` `#link` | Filter from the search box |
| Panel | → | Open or close the item |
| Panel | Ctrl+P | Pin or unpin |
| Panel | F2 or Ctrl+E | Name the item |
| Panel | Ctrl+O | Open with the default app |
| Panel | Delete | Delete the item (when the search box is empty) |
| Panel | Ctrl+1 / Ctrl+2 | Everything / pinned only |
| Panel | F1 | Settings |
| Panel | Esc | Close |

Global shortcuts can be changed in Settings. If another program already holds one, Klipu offers free alternatives.

## Your data

Everything lives in `%LOCALAPPDATA%\Klipu`: the history database, stored images and files, settings and logs. Nothing is sent anywhere. Backups are **not encrypted**, so keep them as carefully as the history itself. Passwords copied from ordinary apps carry no "do not record" mark and are stored like any other text. See [PRIVACY.md](PRIVACY.md).

## Known limitations

- The installer is unsigned, so SmartScreen warns on first run.
- Klipu cannot paste into windows running as administrator unless it also runs as administrator. The item stays on the clipboard so you can paste it yourself.
- OCR uses the languages installed in Windows. Chinese text in images needs the Chinese OCR language feature.
- Search treats CJK text as adjacent characters, so it finds exact substrings but not word variants.

## Build from source

Requirements: Windows 10 or 11, the [Rust toolchain](https://rustup.rs), Visual Studio Build Tools with the C++ workload and a Windows SDK, Node.js 24 and Python 3.

```powershell
cd app
npm ci
cd ..
py scripts/sidecar.py
cd app
npx tauri build --bundles nsis --no-sign
```

The installer lands in `target/release/bundle/nsis/`. To check your work:

```powershell
cargo fmt --all --check
py scripts/sidecar.py --debug
cargo clippy --workspace --all-targets
cargo test --workspace
cd app; npm run lint; npm test
```

Architecture in one paragraph: a Tauri 2 process (`app/src-tauri`, with a React settings window in `app/src`) owns the tray and global shortcuts and starts a separate Slint panel (`crates/cp-panel`), which watches the clipboard and draws the history. `cp-store` is the SQLite and FTS5 history, `cp-core` holds platform-free logic, and `cp-win` and `cp-win-sys` talk to Windows. See [CONTRIBUTING.md](CONTRIBUTING.md) for the conventions.

## License

[GPL-3.0-only](LICENSE). Copyright © 2026 Mario Hidalgo G. (rgdevment) for CoPanda, and © 2026 Klipu contributors for the modifications. This program comes with absolutely no warranty. Third-party components and their licenses are listed in [THIRD-PARTY.md](THIRD-PARTY.md) and inside the app under **About**.

The panel is drawn with [Slint](https://slint.dev), used under the GPL-3.0.

<p>
  <a href="https://slint.dev"><img src="https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png" alt="Made with Slint" height="48"/></a>
</p>
