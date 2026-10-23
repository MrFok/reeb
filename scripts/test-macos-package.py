#!/usr/bin/env python3
"""Verify the actual DMG contents without installing or launching the GUI."""
import argparse
import hashlib
from pathlib import Path
import plistlib
import subprocess
import tempfile
import wave


def run(*args):
    subprocess.run(args, check=True)


def verify(image):
    image = image.resolve(strict=True)
    arch = image.stem.rsplit("-", 1)[-1]
    if arch not in {"arm64", "x86_64"}:
        raise ValueError("Expected an architecture-labelled reeb DMG")
    checksum = image.parent / f"SHA256SUMS-{arch}"
    expected = f"{hashlib.sha256(image.read_bytes()).hexdigest()}  {image.name}\n"
    if checksum.read_text(encoding="utf-8") != expected:
        raise ValueError("DMG checksum does not match its release manifest")
    run("hdiutil", "verify", str(image))
    with tempfile.TemporaryDirectory(prefix="reeb-package-test-") as temporary:
        work = Path(temporary)
        mount = work / "mount"
        mount.mkdir()
        run("hdiutil", "attach", "-readonly", "-nobrowse", "-noautoopen",
            "-mountpoint", str(mount), str(image))
        try:
            visible = sorted(path.name for path in mount.iterdir() if not path.name.startswith("."))
            if visible != ["Applications", "reeb.app"]:
                raise ValueError(f"Unexpected installer contents: {visible}")
            if (mount / "Applications").readlink() != Path("/Applications"):
                raise ValueError("Installer must link to /Applications")
            app = mount / "reeb.app"
            run("codesign", "--verify", "--deep", "--strict", str(app))
            info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
            if info["CFBundleIdentifier"] != "com.mrfok.reeb" or info["CFBundleExecutable"] != "reeb":
                raise ValueError("Incorrect application identity")
            resources = app / "Contents/Resources"
            for path in [mount / ".DS_Store", mount / ".VolumeIcon.icns",
                         mount / ".background/background.png", resources / info["CFBundleIconFile"],
                         resources / "Read Me.md", resources / "Licenses/LICENSE",
                         resources / "Licenses/THIRD-PARTY-NOTICES.txt",
                         resources / "Licenses/SOUND-NOTICES.txt",
                         resources / "Licenses/KBSIM-LICENSE.txt", resources / "Licenses/CC0-LICENSE.txt",
                         resources / "Licenses/PLUS-JAKARTA-SANS-LICENSE.txt"]:
                if not path.is_file() or path.stat().st_size == 0:
                    raise ValueError(f"Missing installer resource: {path}")
            demo = work / "demo.wav"
            run(str(app / "Contents/MacOS/reeb"), "--render-demo", str(demo))
            with wave.open(str(demo)) as audio:
                if audio.getnchannels() != 2 or audio.getsampwidth() != 2 or audio.getnframes() == 0:
                    raise ValueError("Packaged executable must render nonempty 16-bit stereo audio")
                if not any(audio.readframes(audio.getnframes())):
                    raise ValueError("Packaged demo audio is silent")
        finally:
            run("hdiutil", "detach", str(mount))
    print(f"Verified installer, bundled notices and packaged demo: {image.name}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path)
    verify(parser.parse_args().image)
