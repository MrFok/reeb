# Reeb for Windows

Run `reeb-setup-windows-x64.exe` to install for your Windows account, with a Start Menu shortcut and an entry in Installed Apps for uninstalling. No administrator privileges are required. To upgrade, quit reeb from its tray menu, then run the new installer. Preferences are preserved during upgrades and uninstalling. The installer and application are unsigned; Windows may show a SmartScreen warning.

For the portable ZIP, extract it and open `reeb.exe`. Keep the included licenses and notices with the application.

Choose a sound pack in **Switches**. Hover over a switch to preview it, then click to select it. Use the volume and tone controls to adjust the sound. Pause or resume playback with **Keyboard sounds**, the power switch in **Settings**, or the tray menu.

Closing the window keeps Reeb running in the Windows notification area. Click its tray icon to reopen the window, or right-click and choose **Quit Reeb** to exit.

Keyboard events are processed in memory. Reeb does not save typed text or a keystroke history. Preferences are saved locally.

Reeb's source code uses the [MIT License](LICENSE). Bundled recordings, fonts, and dependencies retain their own terms:

- [Dependency notices and license texts](THIRD-PARTY-NOTICES.txt)
- [Recording sources and attribution](SOUND-NOTICES.txt)
- [Recording manifest and hashes](sound-packs.json)
- [MIT recording license](KBSIM-LICENSE.txt)
- [CC0 recording license](CC0-LICENSE.txt)
- [Plus Jakarta Sans font license](PLUS-JAKARTA-SANS-LICENSE.txt)

For a standalone `reeb.exe` download, also download and retain `LICENSE` and `reeb-notices.zip` from the same release. `SHA256SUMS`, available separately with the release, covers the app, installer, both ZIPs, and MIT license.
