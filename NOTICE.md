# Notice

Klipu is a modified version of **CoPanda** (formerly CopyPaste), created by Mario Hidalgo G. (rgdevment):
<https://github.com/rgdevment/CopyPaste>

- Klipu was forked from CoPanda v3.4.0, commit `27f87bf`, on 2 November 2026. The original commit history is preserved in this repository.
- CoPanda is copyright © 2026 Mario Hidalgo G. and is distributed under the GNU General Public License, version 3 only.
- The modifications are copyright © 2026 Klipu contributors and are distributed under the same license. See [LICENSE](LICENSE).
- Klipu is an independent project. It is not affiliated with, endorsed by or supported by the author of CoPanda. "CoPanda" and its artwork belong to their owner and are not used as Klipu's brand.
- CoPanda's separate commercial license and contributor agreement do not apply to Klipu, which is offered only under the GPL-3.0.

## What was changed

**Removed**

- macOS support
- Online update checks and installation
- Import from and removal of CopyPaste 2 data
- "Paste as…" conversions and the global card for them
- Audio waveforms and the `symphonia` dependency, rich media and JSON or token views, and per-kind view switching
- LinkUnbound integration, promotion of other tools, sponsorship links and the Microsoft Store packaging
- Spanish, replaced by Simplified Chinese

**Changed**

- New name, icon, program identity, data folder, startup entry, installer identity and default shortcuts, so that it can run next to CoPanda
- Filters reduced to Text, Image, File and Link
- Search now finds Chinese and Japanese kana from any part of a sentence
- Refined colors, type and status details in the panel and the settings window
- Simplified settings, welcome tour, build, continuous integration and release process

The full list of changes is in the Git history.
