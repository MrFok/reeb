# Project agreements

## Source and assets

Keep this repository limited to independently authored application code, required assets, build/check scripts and current documentation. Preserve source URLs, pinned hashes and applicable licenses for third-party material. Include sound packs when their published licenses permit redistribution, and ship the required attribution and license texts. Add new recordings only with identifiable sources and a documented redistribution grant.

The MIT license covers Reeb's own software. It does not replace third-party asset or dependency licenses or establish ownership of supplied artwork. Consult `assets/brand/README.md` before changing artwork licensing claims.

Keep README prose and headings in the user's lowercase style, preserving technical identifiers and commands. Use the icon alone for the README header image. Retain the requested inspiration acknowledgement. This credit does not authorize importing proprietary code or assets. Tie memory comparisons to the stated observations.

## Windows behavior

With eframe 0.33 / winit 0.30, ordinary repaint requests do not run app updates for a hidden HWND. Preserve the explicit tray wake-up path, including prompt parent repaints while the child popup is open. When changing tray/window lifecycle, build and run `scripts/test-windows-tray.ps1` in an interactive Windows session with other Reeb instances closed. It preserves the existing preferences. Compile and egui tests cannot establish shell behavior.

The tray icon bitmap uses top-down rows. Verify the native HICON when changing image conversion; the executable's icon is a separate path.

Keep the default window size on launch and restore. Geometry checks must enable per-monitor DPI awareness before comparing client dimensions.

## Verification

Use `docs/release-checklist.md` for production preparation. Review the complete change and run the checks relevant to it. Distinguish automated evidence from physical typing, listening and clean-machine tests. Keep keyboard events transient and local. Preserve generation checks on audio events so a delayed producer cannot replay an event from before pause.
