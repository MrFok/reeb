# reeb for macOS

Drag **reeb.app** to **Applications**, then open it there. Builds are architecture-specific: `arm64` is for Apple Silicon; `x86_64` is for Intel. Only Apple Silicon has been exercised locally.

Open **reeb → Settings…** (Command-comma). Under **Background typing**, choose **Enable Input Monitoring…** and allow reeb in **System Settings → Privacy & Security → Input Monitoring**. macOS may require quitting and reopening reeb. If reeb is already enabled but reports no permission, remove its old entry, add the current signed reeb.app, and reopen it. An ad-hoc build has a different code requirement after each rebuild; Developer ID builds retain their signing identity. Until permission is granted, sounds work only while the reeb window is focused. Secure Input, including protected password entry, can prevent background keyboard events.

reeb processes physical key codes in memory to play sounds. It does not read typed text, save a keystroke history, or transmit keyboard events.

The reeb face icon in the menu bar opens reeb, pauses/resumes sounds, or quits. Closing the window keeps sounds running when **Keep running when closed** is enabled. Use **Quit reeb** or **Command-Q** to quit. **Command-W** closes the window; **Command-comma** opens Settings; **Command-H** hides the app.

Appearance follows macOS in **System** mode. The window uses native macOS controls and system text. Sound preferences are saved locally by eframe in the user's application support directory. Audio output is selected in Settings.

## Build

With Rust 1.88 or later, Python 3, and Xcode Command Line Tools already available:

```sh
cargo fetch --locked --target aarch64-apple-darwin
python3 scripts/build-macos.py
```

On an Intel Mac, fetch `x86_64-apple-darwin` instead. The script builds for the installed host toolchain. Outputs are in `dist/macos/`: `reeb.app`, an architecture-labelled DMG, and `SHA256SUMS-arm64` or `SHA256SUMS-x86_64`. Licenses for the Mac dependency graph and recording/font notices are bundled inside the app.

The build automatically uses the installed Developer ID Application identity when exactly one is available. Set `MACOS_SIGN_IDENTITY` to select one explicitly. Without a Developer ID identity, it falls back to ad-hoc signing for local testing; rebuilding an ad-hoc app can invalidate Input Monitoring permission. The build script does not notarize. Use the signing workflow below before public distribution.

## CI and draft releases

The **macOS CI** workflow builds on an Apple Silicon runner for branch pushes, pull requests and manual runs. Download its **reeb-macos-arm64** artifact for the DMG and matching checksum. CI checks formatting, Clippy, tests, bundled assets, the mounted installer layout and licenses, and demo audio rendered by the packaged executable. It does not grant Input Monitoring or test physical typing on the runner.

Version tags run both Windows and Mac packaging before creating the draft release. The Mac builder requires the tag to match `Cargo.toml`. The draft includes the DMG and architecture-specific checksum alongside Windows assets.

GitHub Actions currently has no Developer ID or notarization credentials. CI explicitly uses ad-hoc signing, so its DMGs are test builds. Before publishing a draft, replace its Mac DMG and checksum with the notarized outputs below. Credentials are never embedded in the workflow or uploaded as build artifacts.

To repeat the packaged checks locally:

```sh
python3 scripts/test-macos-package.py dist/macos/reeb-0.1.0-macos-arm64.dmg
```

## Developer ID signing and notarization

The distribution script is pinned to **Developer ID Application: Ricky Fok (DN62F3V62R)**, certificate SHA-1 `88A3863CB01AC3243129E597708798243BD9C892`. This is the personal membership associated with **ricky.y.fok@gmail.com**, verified in Xcode's Apple Accounts settings. It does not select an employer's identity or an Apple Development certificate. Certificate renewal requires updating the pinned fingerprint after checking the new certificate's team.

Build the app, then sign it and the DMG with secure timestamps and the app's hardened runtime:

```sh
python3 scripts/build-macos.py
python3 scripts/sign-macos.py
```

The second command creates a separate Developer ID-signed DMG in `dist/macos/signed/`. It does not notarize unless `--notarize` is specified. Its architecture-specific `signing-*.json` records the signing team, certificate, final hash and notarization state.

For notarization, accept any pending Apple Developer Program agreement for that membership. Store credentials once in your Keychain (the password is entered at a secure prompt, not in the command or repository):

```sh
xcrun notarytool store-credentials reeb-ricky --apple-id ricky.y.fok@gmail.com --team-id DN62F3V62R
```

Use an app-specific password from that Apple account. Then run:

```sh
python3 scripts/sign-macos.py --notarize
```

The script checks the exact installed identity, verifies the app's bundle identifier and signing team, notarizes and staples the app, builds and signs the DMG, then notarizes and staples the DMG. It requires Apple's `Accepted` result and passes both packages through Gatekeeper assessment. Final checksums are calculated after stapling. Credentials stay in the Keychain; submission receipts are retained in the output folder. Signing or notarization failure does not replace an existing DMG. Publication is a separate action. Test the downloaded final package on a clean Mac before publishing.
