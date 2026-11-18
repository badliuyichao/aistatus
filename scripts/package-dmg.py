"""Package the Tauri-built app with a Finder layout that also works on CI."""

import argparse
import json
from pathlib import Path
import plistlib
import subprocess

from dmgbuild import build_dmg


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", nargs="?", help="Rust target passed to Tauri")
    args = parser.parse_args()

    tauri_dir = Path(__file__).resolve().parents[1] / "src-tauri"
    metadata = subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=tauri_dir,
        text=True,
    )
    config = json.loads((tauri_dir / "tauri.conf.json").read_text(encoding="utf-8"))
    product_name = config["productName"]
    target_dir = Path(json.loads(metadata)["target_directory"])
    if args.target:
        target_dir /= args.target
    bundle_dir = target_dir / "release" / "bundle"
    app = bundle_dir / "macos" / f"{product_name}.app"
    with (app / "Contents" / "Info.plist").open("rb") as plist:
        info = plistlib.load(plist)

    executable = app / "Contents" / "MacOS" / info["CFBundleExecutable"]
    archs = subprocess.check_output(["lipo", "-archs", str(executable)], text=True).split()
    suffix = {
        frozenset({"arm64"}): "aarch64",
        frozenset({"x86_64"}): "x64",
        frozenset({"arm64", "x86_64"}): "universal",
    }[frozenset(archs)]
    version = info["CFBundleShortVersionString"]
    output = bundle_dir / "dmg" / f"{product_name}_{version}_{suffix}.dmg"
    output.parent.mkdir(parents=True, exist_ok=True)

    build_dmg(
        filename=str(output),
        volume_name=product_name,
        settings={
            "files": [str(app)],
            "symlinks": {"Applications": "/Applications"},
            "icon": str(tauri_dir / "icons" / "icon.icns"),
            "background": str(tauri_dir / "dmg" / "background.tiff"),
            # The 560×316 pt background sits beneath Finder's title bar.
            "window_rect": ((180, 160), (560, 348)),
            "icon_locations": {app.name: (156, 157), "Applications": (404, 157)},
            "icon_size": 106,
            "text_size": 13,
        },
    )
    print(f"[build] DMG 已生成 {output}")


if __name__ == "__main__":
    main()
