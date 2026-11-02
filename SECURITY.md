# Security

Please report vulnerabilities privately through [GitHub's security advisories](https://github.com/seiya058904/Klipu/security/advisories/new) rather than in a public issue. Include the affected version, the steps to reproduce the problem and its impact.

Only the latest release receives fixes. Klipu is maintained on a best-effort basis, so there is no guaranteed response time.

## Scope

Reports are especially welcome for:

- Clipboard content reaching logs, reports, or anywhere outside the data folder
- Content marked as excluded from clipboard history being stored
- A crafted backup file or clipboard format causing code execution or writing outside the data folder

Klipu stores its history unencrypted under your Windows account, as described in [PRIVACY.md](PRIVACY.md). Another program running as the same user being able to read that history is a known limitation, not a vulnerability.

If an issue also affects [CoPanda](https://github.com/rgdevment/CopyPaste), please consider reporting it to that project too.
