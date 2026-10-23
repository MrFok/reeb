# Release checklist

Run these checks against this repository and its resulting package. A passing build is not a substitute for native Windows checks or listening to the final executable.

## Source and assets

- Keep the repository limited to application source, build/check scripts, required assets, licenses and release documentation. Start with fresh Git history; do not import development archives or unrelated files.
- Confirm that every bundled recording has a published license permitting redistribution and a matching source/hash entry in `assets/sound-packs.json`. The catalog uses 13 profiles from the MIT-licensed kbsim repository and three CC0 profiles from their creators.
- Resolve redistribution rights for the supplied mascot before publishing. The MIT software license does not establish artwork rights; see [brand assets](../assets/brand/README.md).
- Scan the complete source tree for credentials and private paths. Review build-script, workflow, input-hook and persistence changes alongside ordinary application changes.

## Automated checks

Use the toolchain pinned by CI. Python 3.11 or later is needed for maintenance checks, but not for ordinary app builds.

```powershell
cargo fetch --locked --target x86_64-pc-windows-msvc
python scripts/verify-assets.py
python scripts/generate-notices.py --check
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo install cargo-audit --version 0.22.2 --locked
cargo audit --file Cargo.lock --target-os windows --target-arch x86_64 --deny unsound
.\scripts\build-windows.ps1
```

Review advisory results rather than adding blanket ignores. After changing dependencies, regenerate notices with `python scripts/generate-notices.py` and review their license texts. Supplemental upstream license files are pinned and hashed under `assets/dependency-licenses/`.

## Packaged executable

- Verify `dist/SHA256SUMS` against the four named release files. Extract the Windows ZIP into an empty folder and check that it contains only the executable, README and required notices/licenses.
- Render a WAV with the extracted executable: `reeb.exe --render-demo demo.wav`. Confirm that it is nonempty stereo audio with finite, bounded samples.
- Listen to all sound packs, press/release playback, both Enter keys, tone extremes, normalization, spatial width, modifier muting and home-row softness. Test a rapid typing burst and switching packs during playback.
- Test default and named output devices, disconnect/reconnect, refresh and pause/resume. Check that muting clears active and queued sound.

## Windows session

Close other Reeb instances, then run the native lifecycle check in an interactive Windows session. It temporarily preserves/restores existing preferences.

```powershell
.\scripts\test-windows-tray.ps1 -Executable dist/reeb.exe
cargo test --locked popup_wakes_the_hidden_parent_promptly_and_restores_idle_interval -- --ignored
cargo test --locked hover_preview_and_selected_audio_can_share_the_output -- --ignored
```

Also verify physical typing in other apps, repeated Tab while holding Alt, right-click popup controls, focus dismissal, close-to-tray, restart persistence and Quit. Check Light/Dark/System, reduced motion, default window size, small-window scrolling and common Windows display scaling settings.

Use a clean Windows account or VM to verify launch from the downloaded ZIP without Rust or development tools installed. Record the executable hash, OS, audio device and result. Review any SmartScreen warning; the current build is unsigned. Signing is a separate release decision.

## GitHub delivery

Create the new repository from this folder alone. Configure Actions and branch protection, and require the Windows and advisory CI jobs before merging. The workflows use the current repository context, so no owner-specific publishing credentials are embedded.

Every push to `main` increments the patch version (the third number), for example `0.1.1` → `0.1.2`. Update both `Cargo.toml` and the root `reeb` package entry in `Cargo.lock` before pushing.

Set `Cargo.toml` to the intended version and push its matching `vX.Y.Z` tag after approval. The release workflow runs CI, packages the build and prepares the GitHub release files. Review the executable, ZIP, notices and checksums before making a release public. Keep notices with standalone executable downloads; the complete Windows ZIP is the recommended download.

For macOS, follow [Mac signing and notarization](macos.md#developer-id-signing-and-notarization). Only publish the Developer ID-signed, notarized and stapled DMG after confirming its signing record and final checksum. The default Mac builder output is a local test package.

Hosted workflow execution, physical Windows/audio checks and artwork clearance must be recorded separately from local automated checks. Do not label an unrun check as passed.
