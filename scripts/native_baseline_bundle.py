#!/usr/bin/env python3
"""Prepare a local macOS recorder bundle without launching or signing it."""

import argparse
from pathlib import Path
import plistlib
import shutil
import uuid

MODES = ("cold", "pan", "crossing", "gain", "shaders", "local", "two_windows", "idle")


def prepare(binary, mode, output, bundle):
    binary, output, bundle = (Path(path).absolute() for path in (binary, output, bundle))
    if mode not in MODES:
        raise ValueError("unknown workload")
    if not binary.is_file():
        raise ValueError("recorder executable does not exist")
    if output.exists() or output.is_symlink():
        raise ValueError("output already exists")
    if not output.parent.is_dir():
        raise ValueError("output parent does not exist")
    if bundle.suffix != ".app":
        raise ValueError("bundle path must end in .app")
    bundle.mkdir(exist_ok=False)
    executable_dir = bundle / "Contents" / "MacOS"
    executable_dir.mkdir(parents=True)
    shutil.copy2(binary, executable_dir / "rendering_baseline")
    with (bundle / "Contents" / "Info.plist").open("xb") as stream:
        plistlib.dump({
            "CFBundleIdentifier": "dev.radiant.baseline." + uuid.uuid4().hex,
            "CFBundleName": bundle.stem,
            "CFBundleDisplayName": bundle.stem,
            "CFBundleExecutable": "rendering_baseline",
            "CFBundlePackageType": "APPL",
            "CFBundleVersion": "1",
            "CFBundleShortVersionString": "1.0",
            "NSHighResolutionCapable": True,
            "LSEnvironment": {
                "RADIANT_BASELINE_MODE": mode,
                "RADIANT_BASELINE_OUTPUT": str(output),
            },
        }, stream)
    return bundle


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("mode", choices=MODES)
    parser.add_argument("output", type=Path)
    parser.add_argument("bundle", type=Path)
    args = parser.parse_args()
    try:
        print(prepare(args.binary, args.mode, args.output, args.bundle))
    except (OSError, ValueError) as error:
        parser.exit(2, f"{error}\n")
