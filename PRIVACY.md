# Privacy

Klipu has no account, no telemetry, no analytics and no server. **Klipu makes no network requests of its own.** There is no update check. Links in **Settings → About** open in your browser only when you click them. The settings window runs in Microsoft's WebView2, which Windows maintains and updates separately from Klipu.

## What is stored

Everything is kept in `%LOCALAPPDATA%\Klipu` on your computer:

| What | Where |
| :-- | :-- |
| History and search index | `history.db` |
| Larger items such as images | `blobs\` |
| Thumbnails | `thumbs\` |
| Settings | `config.toml` |
| Logs (events and errors, never what you copied) | `logs\` |

Klipu also writes short-lived copies to `%TEMP%\Klipu` when an item has to exist as a file, for example when you drag an image into another app.

For each item, Klipu stores its content and formats, when it was copied and used, the name of the app it came from when known, and any name or pin you gave it. Text inside images is read on your computer by Windows' built-in OCR.

## Sensitive data

Klipu does not read content an app marks as excluded from clipboard history or monitoring. Most password managers do this.

This has limits. A password copied from an ordinary app, such as a text file, carries no such mark and is stored like any other text. The history is **not encrypted**, and neither are backup files. Anyone with access to your Windows account can read them.

## Your control

- **Settings → History** sets how long items are kept and how much space they may use, and can empty the history. Pinned items are kept until you delete them.
- **Settings → About → Save a report** creates a diagnostic file only when you ask. It contains logs and version information, with your user name and home folder blanked out, and it is never sent anywhere.
- To remove everything, uninstall Klipu and choose to delete its data, or quit it and delete the folder above.

## Changes

Changes to this policy are recorded in this repository's Git history.
