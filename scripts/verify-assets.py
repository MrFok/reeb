import hashlib
import json
from pathlib import Path
import wave


ROOT = Path(__file__).resolve().parents[1]


def verify():
    assets = ROOT / "assets"
    manifest = json.loads((assets / "sound-packs.json").read_text(encoding="utf-8"))
    expected = set()
    for pack in manifest["packs"]:
        if pack["license"] not in {"CC0-1.0", "MIT"} or not pack.get("license_evidence"):
            raise ValueError(f"Recording permission needs review: {pack['id']}")
        for item in pack["files"]:
            path = (assets / item["file"]).resolve()
            if not path.is_relative_to(assets.resolve()) or path in expected:
                raise ValueError(f"Invalid or duplicate asset path: {item['file']}")
            expected.add(path)
            if hashlib.sha256(path.read_bytes()).hexdigest() != item["wav_sha256"]:
                raise ValueError(f"Recording hash mismatch: {item['file']}")
            if not item.get("source") or len(item.get("source_sha256", "")) != 64:
                raise ValueError(f"Missing source provenance: {item['file']}")
            with wave.open(str(path)) as sound:
                if sound.getparams()[:3] != (1, 2, 44100) or sound.getnframes() < 2:
                    raise ValueError(f"Invalid recording format: {item['file']}")
    actual = {path.resolve() for path in assets.rglob("*.wav")}
    if actual != expected:
        raise ValueError("Bundled recordings and manifest differ")
    fonts = json.loads((assets / "ui/provenance.json").read_text(encoding="utf-8"))
    for name, digest in fonts["files"].items():
        if hashlib.sha256((assets / "ui" / name).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Font hash mismatch: {name}")
    print(f"Verified {len(manifest['packs'])} packs / {len(expected)} recordings and font hashes")


if __name__ == "__main__":
    verify()
