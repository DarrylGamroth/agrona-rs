#!/usr/bin/env python3
"""Generate counter-region fixtures with the pinned Agrona Java sources."""

from __future__ import annotations

import argparse
import pathlib
import shutil
import subprocess
import tempfile


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--agrona-root",
        required=True,
        type=pathlib.Path,
        help="path to an Agrona repository at the pinned revision",
    )
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args()

    repository = pathlib.Path(__file__).resolve().parents[1]
    generator = (
        repository
        / "tests"
        / "interop"
        / "java"
        / "CountersReaderFixtureGenerator.java"
    )
    args.output.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="agrona-counter-fixture-") as temporary:
        temporary_path = pathlib.Path(temporary)
        build_root = temporary_path / "agrona"
        shutil.copytree(
            args.agrona_root,
            build_root,
            ignore=shutil.ignore_patterns(
                ".git", ".gradle", "bin", ".idea", ".vscode"
            ),
        )
        subprocess.run(
            [
                str(build_root / "gradlew"),
                ":agrona:jar",
                "--no-daemon",
                "--rerun-tasks",
            ],
            cwd=build_root,
            check=True,
        )
        jars = [
            path
            for path in (build_root / "agrona" / "build" / "libs").glob("agrona-*.jar")
            if not path.name.endswith(("-sources.jar", "-javadoc.jar"))
        ]
        if len(jars) != 1:
            raise RuntimeError(f"expected one Agrona runtime jar, found: {jars}")

        classes = temporary_path / "fixture-classes"
        classes.mkdir()
        subprocess.run(
            [
                "javac",
                "--release",
                "17",
                "-d",
                str(classes),
                "-cp",
                str(jars[0]),
                str(generator),
            ],
            check=True,
        )
        subprocess.run(
            [
                "java",
                "--add-opens",
                "java.base/jdk.internal.misc=ALL-UNNAMED",
                "-cp",
                f"{classes}:{jars[0]}",
                "CountersReaderFixtureGenerator",
                str(args.output),
            ],
            check=True,
        )


if __name__ == "__main__":
    main()
