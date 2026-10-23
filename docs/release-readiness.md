# Local release verification

Verified on Windows x64 on 2026-10-22 using Rust 1.95.0. These results apply to the prepared 0.1.0 export. Hosted CI and release publication are separate from these local checks.

| Check | Result |
| --- | --- |
| Formatting | Passed |
| Clippy, all targets, warnings denied | Passed |
| Default Rust suite | 62 passed; 2 native tests excluded by default |
| Native class/window icon assignment | Passed with a hidden window; Task Manager appearance still needs visual confirmation |
| Hidden-parent tray wake-up helper | Passed separately |
| Muted native hover-audio/selected-stream check | Passed separately, including after the pause fix |
| Sound and font hashes | Passed: 16 profiles, 187 recordings |
| Locked Windows dependency notices | Regenerated and checked |
| RustSec Windows x64 audit, unsoundness denied | No known vulnerabilities or unsoundness findings; one unmaintained dependency warning |
| Release build and package | Passed; x64 executable and explicit package inventory verified |
| Extracted executable demo | Passed: 115,200 stereo frames at 48 kHz, peak 0.514404, RMS 0.008777 |
| Checksums, ZIP contents and packaged README links | Passed |
| Source/asset scan for excluded product references | No code/asset matches; the README inspiration credit is intentionally retained |
| High-signal credential patterns | No matches; a pattern scan is not a comprehensive security audit |

The executable is 9,192,448 bytes. SHA-256:

```text
a4f12d4de3061bf22d710b4c04aa9c670db42ec9b6dbb5089024bd13cc2cec8d
```

The dependency audit used cargo-audit 0.22.2 and advisory database commit `640bf5a9f06e66d8721e8772766531fe77c8630d`. Warning `RUSTSEC-2026-0192` reports that indirect dependency `ttf-parser` 0.25.1 is unmaintained. It is not a reported vulnerability. Track its replacement through the GUI/font dependencies rather than suppressing the warning.

Independent reviews identified and resolved stale audio events across pause/resume, valid-event loss across the same transition, disconnected-keyboard visualizer state and host-dependent notice ordering. The new sound processing and key-position model use independently specified tuning. Listening to the final tuning remains a release check.

Still required before a public release:

- Run the full interactive tray/window lifecycle script against the final executable; the hidden helper test covers only part of that flow.
- Perform physical typing, repeated Alt+Tab, popup interaction, display-scaling and audio listening/device-recovery checks.
- Verify the downloaded package in a clean Windows account or VM without developer tools installed.
- Resolve supplied artwork redistribution rights. Keeping the icon is the project owner's decision; the software's MIT license does not establish artwork ownership.
- Run the hosted workflows in the new repository and review the resulting draft. The executable is currently unsigned; signing and SmartScreen behavior remain release decisions/checks.

The local harness's workflow fixtures and cost/accounting evaluation test the coding process itself. They are not product acceptance tests or a production certification gate. The applicable product process is the [release checklist](release-checklist.md): source and license review, automated checks, native verification, package verification and review before publishing.
