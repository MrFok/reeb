import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parents[1]


def generate(target="x86_64-pc-windows-msvc"):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1",
         "--filter-platform", target], cwd=ROOT, text=True,
    ))
    selected = {node["id"] for node in metadata["resolve"]["nodes"]}
    packages = sorted((package for package in metadata["packages"]
                       if package["id"] in selected and package["name"] != "reeb"),
                      key=lambda package: (package["name"], package["version"]))
    graph = "Windows" if target == "x86_64-pc-windows-msvc" else target
    sections = ["THIRD-PARTY NOTICES\n\n"
                f"License texts for the locked {graph} dependency graph, including build tools.\n"
                "A package's presence does not imply all of its optional features are linked.\n"]
    supplements = ROOT / "assets" / "dependency-licenses"
    sources = json.loads((supplements / "sources.json").read_text(encoding="utf-8"))
    for package in packages:
        directory = Path(package["manifest_path"]).parent
        files = set()
        if package.get("license_file"):
            files.add(directory / package["license_file"])
        for path in directory.rglob("*"):
            if path.is_file() and re.search(r"(^|[-_.])(licen[cs]e|copying|notice|copyright|ofl|ufl)([-_.]|$)", path.name, re.I):
                if path.suffix.lower() not in {".rs", ".toml", ".json"}:
                    files.add(path)
        if package["name"] == "epaint_default_fonts":
            files.update((directory / "fonts").glob("*.txt"))
        extra = sources.get(f"{package['name']} {package['version']}", [])
        if not files and not extra:
            raise ValueError(f"Missing license text: {package['name']} {package['version']}")
        sections.append("\n" + "=" * 72 + "\n"
                        f"{package['name']} {package['version']}\n"
                        f"Declared license: {package.get('license') or 'See included license'}\n"
                        f"Source: https://crates.io/crates/{package['name']}/{package['version']}\n")
        for path in sorted(files, key=lambda item: item.relative_to(directory).as_posix().casefold()):
            sections.append(f"\n{path.relative_to(directory).as_posix()}\n"
                            + path.read_text(encoding="utf-8-sig") + "\n")
        for entry in extra:
            path = supplements / entry["file"]
            if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                raise ValueError(f"Supplemental license hash mismatch: {path.name}")
            sections.append(f"\n{entry['file']}\nSource: {entry['source']}\n"
                            + path.read_text(encoding="utf-8-sig") + "\n")
    return "".join(sections)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--target", default="x86_64-pc-windows-msvc")
    parser.add_argument("--output", type=Path, default=ROOT / "THIRD-PARTY-NOTICES.txt")
    args = parser.parse_args()
    result = generate(args.target)
    destination = args.output
    if args.check:
        if destination.read_text(encoding="utf-8") != result:
            raise SystemExit("Dependency notices are stale; run scripts/generate-notices.py")
        print(f"Dependency notices match the locked {args.target} graph")
    else:
        destination.write_text(result, encoding="utf-8")
        print(f"Wrote {destination.name}")
