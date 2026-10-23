<p align="center">
  <img src="assets/brand/reeb-icon-256.png" width="96" alt="reeb" />
</p>

# reeb

mechanical keyboard sounds for windows and macOS, built in rust.

choose from over 15 keyboard sound packs and shape your keyboard's tone and feel.

reeb is **free**, and available for windows, macos, & linux.

## memory usage

![windows idle memory usage: Keeby 502.4 MB; reeb 132.9 MB, 73.5% less resident ram.](assets/benchmarks/memory-comparison.svg)

[data and comparison details](docs/memory-comparison.md).

## download

[download reeb.exe for windows (x64)](https://github.com/MrFok/reeb/releases/latest/download/reeb.exe)

open `reeb.exe` to start. no installer or administrator privileges are required. also download and retain the [license](https://github.com/MrFok/reeb/releases/latest/download/LICENSE) and [third-party notices](https://github.com/MrFok/reeb/releases/latest/download/reeb-notices.zip) with the app.

## macOS

Build a native Mac app and DMG with `python3 scripts/build-macos.py` after fetching the locked Mac dependencies. The app includes native window controls, a menu-bar icon, Command-Q / Command-comma, system appearance and background typing with Input Monitoring permission. See [Mac setup and packaging](docs/macos.md).

Mac packaging uses an installed Developer ID identity when available; otherwise it produces an ad-hoc test build. Notarization is a separate step before distributing a trusted download. Apple Silicon is tested; Intel requires its own build and verification.

For distribution, `python3 scripts/sign-macos.py --notarize` uses the configured personal Developer ID identity and Keychain credentials to produce a signed, notarized and stapled DMG in `dist/macos/signed/`. See the [signing setup](docs/macos.md#developer-id-signing-and-notarization).

The **macOS CI** workflow uploads the Apple Silicon DMG and checksum as **reeb-macos-arm64**. Version-tag builds also attach them to the draft release. CI DMGs are ad-hoc test builds; replace them with the notarized package before publishing a release.

## get started

1. open **Switches**. hover over a switch to hear three keystrokes, then click to select it.
2. type in any app. adjust volume, tone, spatial positioning and typing feel in reeb.
3. pause or resume with **Keyboard sounds**, the power switch in **Settings**, or the tray menu.

closing the window keeps reeb running in the windows notification area, sometimes behind the hidden-icons arrow. click its icon to reopen it; right-click for quick controls and settings. choose **Quit Reeb** to stop it completely.

## privacy

**reeb does not record your keystrokes.**

reeb processes keyboard events in memory to play sounds and animate the visualizer. it doesn't save typed text or a keystroke history, and it doesn't send your typing anywhere. your sound and appearance preferences stay on your computer.

## build from source

install Rust 1.88 or later with the MSVC toolchain and Visual Studio C++ Build Tools with the Windows SDK. from the repository folder, run:

```powershell
.\scripts\build-windows.ps1
```

dependencies are locked in `Cargo.lock`. the build uses the checked-in assets, so you don't need python or asset-generation tools.

for development:

```sh
cargo run --locked
```

## development and releases

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

ci checks the source and bundled assets. the release workflow builds the windows executable and packages it with licenses and notices in a ZIP. it also generates SHA-256 checksums. see the [release checklist](docs/release-checklist.md) for verification and publishing steps.

## license

reeb's source code is available under the [MIT License](LICENSE). third-party recordings, fonts and dependencies retain their own licenses and notices.

## acknowledgements

- [kbsim](https://github.com/tplai/kbsim) by thomas lai for the switch recordings distributed under its MIT license.
- [foxfire-](https://freesound.org/people/Foxfire-/) and [unicaegames](https://opengameart.org/content/keyboard-soundpack-1-typing-and-single-keystrokes) for CC0 keyboard recordings.
- [sadiquecat](https://freesound.org/people/Sadiquecat/sounds/799110/) for the CC0 MacBook Air M2 recordings.
- [plus jakarta sans](https://github.com/tokotype/PlusJakartaSans) for the interface typeface, under the [SIL Open Font License](assets/ui/OFL.txt).
- the rust projects that power reeb, including [egui/eframe](https://github.com/emilk/egui), [CPAL](https://github.com/RustAudio/cpal) and [Hound](https://github.com/ruuda/hound).
- [keeby](https://getkeeby.com/) as the main inspiration for this project.

recording sources and hashes are listed in the [sound manifest](assets/sound-packs.json), with attribution in [sound notices](assets/SOUND-NOTICES.txt). dependency license texts are in [third-party notices](THIRD-PARTY-NOTICES.txt).
