"""Native, dependency-free Finder layout for reeb's drag-to-install DMG."""
from pathlib import Path
import plistlib
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def finder_store(destination, background_alias):
    """Write the small single-leaf DSDB used by this fixed two-item layout.

    Format references: dmgbuild/ds_store's ILocCodec and the Bud1/DSDB file
    layout documented by sindresorhus/DSStore. No general-purpose DB editing.
    """
    records = []

    def record(name, code, kind, value):
        if kind == b"blob":
            value = struct.pack(">I", len(value)) + value
        text = name.encode("utf-16be")
        records.append((name.casefold(), code,
                        struct.pack(">I", len(text) // 2) + text + code + kind + value))

    def plist(code, value):
        record(".", code, b"blob", plistlib.dumps(value, fmt=plistlib.FMT_BINARY))

    plist(b"bwsp", {"WindowBounds": "{{180, 160}, {640, 422}}", "ShowSidebar": False,
                    "ContainerShowSidebar": False, "ShowToolbar": False, "ShowStatusBar": False,
                    "ShowPathbar": False, "ShowTabView": False, "SidebarWidth": 0})
    plist(b"icvp", {"viewOptionsVersion": 1, "backgroundType": 2,
                    "backgroundImageAlias": background_alias, "iconSize": 112.0,
                    "textSize": 13.0, "gridSpacing": 100.0, "gridOffsetX": 0.0,
                    "gridOffsetY": 0.0, "labelOnBottom": True, "arrangeBy": "none",
                    "showIconPreview": True, "showItemInfo": False,
                    "backgroundColorRed": 0.96, "backgroundColorGreen": 0.97,
                    "backgroundColorBlue": 0.99})
    record(".", b"vstl", b"type", b"icnv")
    record(".", b"vSrn", b"long", struct.pack(">I", 1))
    for name, x in [("reeb.app", 160), ("Applications", 480)]:
        record(name, b"Iloc", b"blob", struct.pack(">IIII", x, 216, 0xFFFFFFFF, 0xFFFF0000))
    leaf = struct.pack(">II", 0, len(records)) + b"".join(r[2] for r in sorted(records))
    if len(leaf) > 4096:
        raise ValueError("DMG Finder metadata exceeds its single 4096-byte page")
    data = bytearray(16388)
    struct.pack_into(">I4sIII16x", data, 0, 1, b"Bud1", 8192, 2048, 8192)
    struct.pack_into(">IIIII", data, 68, 2, 0, len(records), 1, 4096)
    data[4100:4100 + len(leaf)] = leaf
    # Buddy allocator: root (2048), DSDB header (32), leaf (4096).
    allocator = struct.pack(">II", 3, 0) + struct.pack(">III", 8192 | 11, 64 | 5, 4096 | 12)
    allocator += bytes(4 * 253) + struct.pack(">IB4sI", 1, 4, b"DSDB", 1)
    free = {5: [32, 96], 7: [128], 8: [256], 9: [512], 10: [1024],
            11: [2048, 10240], 12: [12288]}
    for power in range(32):
        blocks = free.get(power, [1 << power] if 14 <= power <= 30 else [])
        allocator += struct.pack(">I", len(blocks)) + b"".join(struct.pack(">I", b) for b in blocks)
    if len(allocator) > 2048:
        raise ValueError("DMG allocator metadata exceeds its block")
    data[8196:8196 + len(allocator)] = allocator
    destination.write_bytes(data)


def create_image(stage, destination):
    stage, destination = Path(stage), Path(destination)
    with tempfile.TemporaryDirectory(prefix="reeb-dmg-") as temporary:
        work = Path(temporary)
        helper = work / "macos-dmg"
        run("xcrun", "clang", "-fobjc-arc", "-Wno-deprecated-declarations",
            str(ROOT / "scripts/macos-dmg.m"), "-framework", "AppKit", "-framework",
            "CoreServices", "-o", str(helper))
        run(str(helper), "assets", str(ROOT / "assets/brand/reeb-icon-256.png"),
            str(stage / "reeb.app/Contents/Resources/reeb.icns"), str(stage))
        image = work / "layout.dmg"
        run("hdiutil", "create", "-volname", "reeb", "-srcfolder", str(stage),
            "-fs", "HFS+", "-format", "UDRW", str(image))
        mount = work / "mount"
        mount.mkdir()
        run("hdiutil", "attach", "-nobrowse", "-noautoopen", "-mountpoint", str(mount), str(image),
            stdout=subprocess.DEVNULL)
        try:
            alias = work / "background.alias"
            run(str(helper), "alias", str(mount), str(alias))
            finder_store(mount / ".DS_Store", alias.read_bytes())
            run("xcrun", "SetFile", "-a", "C", str(mount))
        finally:
            run("hdiutil", "detach", str(mount), stdout=subprocess.DEVNULL)
        run("hdiutil", "convert", str(image), "-format", "UDZO", "-ov", "-o", str(destination))
