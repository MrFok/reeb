#!/usr/bin/env python3
"""Sign reeb with Ricky's Developer ID and optionally notarize the DMG."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import tempfile

from macos_dmg import create_image

ROOT = Path(__file__).resolve().parents[1]
APPLE_ID = "ricky.y.fok@gmail.com"
TEAM_ID = "DN62F3V62R"
IDENTITY = "88A3863CB01AC3243129E597708798243BD9C892"
BUNDLE_ID = "com.mrfok.reeb"


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def verify_signature(path):
    run("codesign", "--verify", "--deep", "--strict", str(path))
    result = run("codesign", "--display", "--verbose=4", str(path),
                 capture_output=True, text=True)
    details = result.stderr.splitlines()
    if f"TeamIdentifier={TEAM_ID}" not in details:
        raise SystemExit(f"Refusing a signature outside {APPLE_ID}'s team {TEAM_ID}.")
    if not any(line.startswith("Authority=Developer ID Application: Ricky Fok (") for line in details):
        raise SystemExit("Expected Ricky Fok's Developer ID Application certificate.")
    if not any(line.startswith("Timestamp=") for line in details):
        raise SystemExit("The Developer ID signature must have a secure timestamp.")


def sign(app, notarize, profile):
    if platform.system() != "Darwin":
        raise SystemExit("Signing and notarization require macOS.")
    app = app.resolve(strict=True)
    with (app / "Contents/Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if info.get("CFBundleIdentifier") != BUNDLE_ID or info.get("CFBundleExecutable") != "reeb":
        raise SystemExit("Expected a reeb.app built by scripts/build-macos.py.")
    version = info["CFBundleShortVersionString"]
    arch = subprocess.check_output(
        ["lipo", "-archs", str(app / "Contents/MacOS/reeb")], text=True).strip()
    if arch not in {"arm64", "x86_64"}:
        raise SystemExit(f"Expected one supported architecture, got {arch}.")
    # Validate the exact certificate before creating or replacing any output.
    identities = subprocess.check_output(["security", "find-identity", "-v", "-p", "codesigning"], text=True)
    if IDENTITY not in identities:
        raise SystemExit(f"Install the Developer ID Application identity for {APPLE_ID}, team {TEAM_ID}.")
    if notarize:
        # Authenticate before modifying a release. Credentials stay in the Keychain.
        run("xcrun", "notarytool", "history", "--keychain-profile", profile,
            "--output-format", "json", stdout=subprocess.DEVNULL)
    destination = ROOT / "dist/macos/signed"
    destination.mkdir(parents=True, exist_ok=True)
    image = destination / f"reeb-{version}-macos-{arch}.dmg"
    with tempfile.TemporaryDirectory(prefix="reeb-sign-") as temporary:
        work = Path(temporary)
        stage = work / "image"
        stage.mkdir()
        signed_app = stage / "reeb.app"
        shutil.copytree(app, signed_app, symlinks=True)
        shutil.copy2(ROOT / "docs/macos.md", signed_app / "Contents/Resources/Read Me.md")
        run("codesign", "--force", "--sign", IDENTITY, "--options", "runtime",
            "--timestamp", str(signed_app))
        verify_signature(signed_app)
        (stage / "Applications").symlink_to("/Applications")
        # Notarize the app first so its ticket survives dragging it out of the DMG.
        if notarize:
            archive = work / "reeb.zip"
            run("ditto", "-c", "-k", "--keepParent", str(signed_app), str(archive))
            submit(archive, profile, work / "app-notarization.json")
            run("xcrun", "stapler", "staple", str(signed_app))
            run("xcrun", "stapler", "validate", str(signed_app))
            run("spctl", "--assess", "--type", "execute", "--verbose=2", str(signed_app))
        temporary_image = work / image.name
        create_image(stage, temporary_image)
        run("codesign", "--sign", IDENTITY, "--timestamp", str(temporary_image))
        verify_signature(temporary_image)
        if notarize:
            submit(temporary_image, profile, work / "dmg-notarization.json")
            run("xcrun", "stapler", "staple", str(temporary_image))
            run("xcrun", "stapler", "validate", str(temporary_image))
            run("spctl", "--assess", "--type", "open", "--context",
                "context:primary-signature", "--verbose=2", str(temporary_image))
        run("hdiutil", "verify", str(temporary_image))
        shutil.copy2(temporary_image, image)
        # Preserve Apple's submission IDs and results without credentials.
        for receipt in work.glob("*-notarization.json"):
            shutil.copy2(receipt, destination / receipt.name)
    digest = hashlib.sha256(image.read_bytes()).hexdigest()
    # Each architecture has its own checksum file, so one build cannot hide another.
    (destination / f"SHA256SUMS-{arch}").write_text(f"{digest}  {image.name}\n", encoding="utf-8")
    (destination / f"signing-{arch}.json").write_text(json.dumps({
        "file": image.name, "sha256": digest, "team_id": TEAM_ID,
        "identity_sha1": IDENTITY, "notarized": notarize,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"Built {image}\nDeveloper ID: Ricky Fok ({TEAM_ID})\nNotarized: {notarize}")
    if not notarize:
        print("Signed only: run again with --notarize before public distribution.")


def submit(path, profile, receipt):
    result = subprocess.run(
        ["xcrun", "notarytool", "submit", str(path), "--keychain-profile", profile,
         "--wait", "--output-format", "json"], capture_output=True, text=True)
    receipt.write_text(result.stdout, encoding="utf-8")
    try:
        status = json.loads(result.stdout)
    except json.JSONDecodeError:
        status = {}
    if result.returncode != 0 or status.get("status") != "Accepted":
        # Keep diagnostics even though the temporary package will be discarded.
        failure = ROOT / "dist/macos" / receipt.name
        shutil.copy2(receipt, failure)
        raise SystemExit(f"Notarization was not accepted; see {failure}. Submission: {status.get('id')}. {result.stderr.strip()}")
    print(f"Apple accepted {path.name}: {status['id']}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=ROOT / "dist/macos/reeb.app")
    parser.add_argument("--notarize", action="store_true")
    parser.add_argument("--keychain-profile", default="reeb-ricky")
    args = parser.parse_args()
    sign(args.app, args.notarize, args.keychain_profile)
