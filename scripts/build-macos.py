#!/usr/bin/env python3
"""Build a host-architecture macOS app and drag-to-Applications disk image."""
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import tempfile

from macos_dmg import create_image

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, **kwargs)


def build():
    if platform.system() != "Darwin":
        raise SystemExit("Build on macOS with Rust and Xcode Command Line Tools.")
    if os.environ.get("CARGO_BUILD_TARGET"):
        raise SystemExit("Unset CARGO_BUILD_TARGET; this script packages the host architecture.")
    host = subprocess.check_output(["rustc", "-vV"], text=True)
    target = next(line.removeprefix("host: ") for line in host.splitlines() if line.startswith("host: "))
    arch = {"aarch64-apple-darwin": "arm64", "x86_64-apple-darwin": "x86_64"}.get(target)
    if arch is None:
        raise SystemExit(f"Unsupported Rust host: {target}")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"], cwd=ROOT))
    package = next(p for p in metadata["packages"] if p["name"] == "reeb")
    version = package["version"]
    tag = os.environ.get("RELEASE_TAG")
    if tag and tag != f"v{version}":
        raise SystemExit(f"Release tag {tag} does not match package version {version}.")
    run("python3", "scripts/verify-assets.py")
    run("cargo", "build", "--release", "--locked", "--offline", "--bin", "reeb")
    executable = Path(metadata["target_directory"]) / "release" / "reeb"
    actual = subprocess.check_output(["lipo", "-archs", str(executable)], text=True).strip()
    if actual != arch:
        raise SystemExit(f"Expected {arch} binary; got {actual}")
    destination = ROOT / "dist" / "macos"
    destination.mkdir(parents=True, exist_ok=True)
    image = destination / f"reeb-{version}-macos-{arch}.dmg"
    signing_identity = os.environ.get("MACOS_SIGN_IDENTITY")
    if signing_identity is None:
        identities = subprocess.check_output(["security", "find-identity", "-v", "-p", "codesigning"], text=True)
        developer_ids = [line.split('"')[1] for line in identities.splitlines()
                         if '"Developer ID Application:' in line]
        if len(developer_ids) > 1:
            raise SystemExit("Choose the signing identity with MACOS_SIGN_IDENTITY.")
        signing_identity = developer_ids[0] if developer_ids else "-"
    with tempfile.TemporaryDirectory(prefix="reeb-macos-") as temporary:
        work = Path(temporary)
        stage = work / "image"
        contents = stage / "reeb.app" / "Contents"
        macos = contents / "MacOS"
        resources = contents / "Resources"
        macos.mkdir(parents=True)
        resources.mkdir()
        shutil.copy2(executable, macos / "reeb")
        with (contents / "Info.plist").open("wb") as stream:
            plistlib.dump({
                "CFBundleName": "reeb", "CFBundleDisplayName": "reeb",
                "CFBundleIdentifier": "com.mrfok.reeb", "CFBundleExecutable": "reeb",
                "CFBundlePackageType": "APPL", "CFBundleIconFile": "reeb.icns",
                "CFBundleShortVersionString": version, "CFBundleVersion": version,
                "LSMinimumSystemVersion": "11.0", "NSHighResolutionCapable": True,
                "NSSupportsAutomaticTermination": False,
                "NSHumanReadableCopyright": "See bundled licenses and notices.",
                "NSInputMonitoringUsageDescription": "reeb listens to physical key presses to play keyboard sounds. Typed text is never stored or sent.",
            }, stream)
        iconset = work / "reeb.iconset"
        iconset.mkdir()
        source = ROOT / "assets" / "brand" / "reeb-icon-256.png"
        run("xcrun", "swift", "scripts/macos-icon.swift", str(source), str(iconset))
        run("iconutil", "-c", "icns", str(iconset), "-o", str(resources / "reeb.icns"))
        licenses = resources / "Licenses"
        licenses.mkdir()
        for name in ["LICENSE", "assets/KBSIM-LICENSE.txt", "assets/CC0-LICENSE.txt",
                     "assets/SOUND-NOTICES.txt", "assets/sound-packs.json"]:
            shutil.copy2(ROOT / name, licenses / Path(name).name)
        shutil.copy2(ROOT / "assets/ui/OFL.txt", licenses / "PLUS-JAKARTA-SANS-LICENSE.txt")
        run("python3", "scripts/generate-notices.py", "--target", target,
            "--output", str(licenses / "THIRD-PARTY-NOTICES.txt"))
        shutil.copy2(ROOT / "docs/macos.md", resources / "Read Me.md")
        (stage / "Applications").symlink_to("/Applications")
        signing = ["codesign", "--force", "--sign", signing_identity, "--options", "runtime"]
        signing += ["--timestamp=none"] if signing_identity == "-" else ["--timestamp"]
        run(*signing, str(stage / "reeb.app"))
        run("codesign", "--verify", "--deep", "--strict", str(stage / "reeb.app"))
        create_image(stage, image)
        run("hdiutil", "verify", str(image))
        # Keep a directly runnable app alongside the DMG, replacing only this generated bundle.
        app = destination / "reeb.app"
        if app.exists():
            shutil.rmtree(app)
        shutil.copytree(stage / "reeb.app", app)
    digest = hashlib.sha256(image.read_bytes()).hexdigest()
    (destination / f"SHA256SUMS-{arch}").write_text(f"{digest}  {image.name}\n", encoding="utf-8")
    print(f"Built {image}\nApp: {app}\nSigning: {signing_identity}")
    if signing_identity == "-":
        print("Local test build: ad-hoc signed, not notarized. Developer ID signing is needed for distribution.")


if __name__ == "__main__":
    build()
