#!/usr/bin/env python3
"""Validate release version consistency across Cargo, bundle, binary, and tag."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Sequence

PACKAGE_NAME = "gpssim"
TAG_REF_PREFIX = "refs/tags/"


class VersionInvariantError(Exception):
    """Raised when release version sources disagree or cannot be inspected."""


def run_command(command: Sequence[str]) -> str:
    """Run a command and return stdout, preserving useful failure diagnostics."""
    try:
        result = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as error:
        raise VersionInvariantError(
            f"failed to execute {command[0]!r}: {error}"
        ) from error

    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "no diagnostic output"
        raise VersionInvariantError(
            f"{' '.join(command)} failed with exit code {result.returncode}: {detail}"
        )

    return result.stdout


def package_version(metadata: Any, package_name: str = PACKAGE_NAME) -> str:
    """Extract one package version and require cargo-bundle to inherit it."""
    if not isinstance(metadata, dict):
        raise VersionInvariantError("Cargo metadata root is not an object")

    packages = metadata.get("packages")
    if not isinstance(packages, list):
        raise VersionInvariantError("Cargo metadata does not contain a package list")

    matching_packages = [
        package
        for package in packages
        if isinstance(package, dict) and package.get("name") == package_name
    ]
    if len(matching_packages) != 1:
        raise VersionInvariantError(
            f"expected exactly one {package_name!r} package, found {len(matching_packages)}"
        )

    package = matching_packages[0]
    version = package.get("version")
    if not isinstance(version, str) or not version:
        raise VersionInvariantError(f"{package_name!r} has no valid Cargo version")

    package_metadata = package.get("metadata", {})
    if not isinstance(package_metadata, dict):
        raise VersionInvariantError(f"{package_name!r} package metadata is not an object")

    bundle_metadata = package_metadata.get("bundle", {})
    if not isinstance(bundle_metadata, dict):
        raise VersionInvariantError(f"{package_name!r} bundle metadata is not an object")
    if "version" in bundle_metadata:
        raise VersionInvariantError(
            "package.metadata.bundle.version must be omitted so cargo-bundle uses "
            "the Cargo package version"
        )

    return version


def binary_version(output: str, package_name: str = PACKAGE_NAME) -> str:
    """Extract the exact version emitted by the built binary."""
    lines = output.strip().splitlines()
    if len(lines) != 1:
        raise VersionInvariantError(
            f"{package_name} --version must emit exactly one non-empty line"
        )

    prefix = f"{package_name} "
    if not lines[0].startswith(prefix):
        raise VersionInvariantError(
            f"unexpected {package_name} --version output: {lines[0]!r}"
        )

    version = lines[0][len(prefix) :]
    if (
        not version
        or version.strip() != version
        or any(character.isspace() for character in version)
    ):
        raise VersionInvariantError(
            f"unexpected {package_name} binary version: {version!r}"
        )

    return version


def tag_from_github_ref(github_ref: str | None) -> str | None:
    """Return a tag name only when GitHub is executing a tag ref."""
    if github_ref and github_ref.startswith(TAG_REF_PREFIX):
        tag = github_ref[len(TAG_REF_PREFIX) :]
        if not tag:
            raise VersionInvariantError("GitHub tag ref has an empty tag name")
        return tag
    return None


def validate_versions(
    cargo_version: str,
    executable_version: str,
    expected_tag: str | None,
) -> None:
    """Enforce equality between every applicable release version source."""
    if executable_version != cargo_version:
        raise VersionInvariantError(
            f"binary version {executable_version!r} does not match Cargo version "
            f"{cargo_version!r}"
        )

    if expected_tag is not None:
        required_tag = f"v{cargo_version}"
        if expected_tag != required_tag:
            raise VersionInvariantError(
                f"release tag {expected_tag!r} does not match required tag {required_tag!r}"
            )


def check_release_version(binary_path: Path, expected_tag: str | None) -> str:
    """Inspect Cargo metadata and a built binary, then validate all versions."""
    metadata_output = run_command(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"]
    )
    try:
        metadata = json.loads(metadata_output)
    except json.JSONDecodeError as error:
        raise VersionInvariantError(f"cargo metadata returned invalid JSON: {error}") from error

    cargo_version = package_version(metadata)
    executable_version = binary_version(run_command([str(binary_path), "--version"]))
    validate_versions(cargo_version, executable_version, expected_tag)
    return cargo_version


def default_binary_path() -> Path:
    """Return the default release binary path for the current host."""
    executable_name = "gpssim.exe" if os.name == "nt" else "gpssim"
    return Path("target") / "release" / executable_name


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--binary",
        type=Path,
        default=default_binary_path(),
        help="path to the built gpssim executable",
    )
    parser.add_argument(
        "--tag",
        help="release tag to validate; tag refs are otherwise read from GITHUB_REF",
    )
    return parser.parse_args()


def main() -> int:
    """Run the release version invariant check."""
    args = parse_args()
    try:
        github_tag = tag_from_github_ref(os.environ.get("GITHUB_REF"))
        if args.tag is not None and github_tag is not None and args.tag != github_tag:
            raise VersionInvariantError(
                f"explicit tag {args.tag!r} does not match GitHub tag {github_tag!r}"
            )

        expected_tag = args.tag if args.tag is not None else github_tag
        version = check_release_version(args.binary, expected_tag)
    except VersionInvariantError as error:
        print(f"release version invariant failed: {error}", file=sys.stderr)
        return 1

    tag_summary = expected_tag if expected_tag is not None else "not a tag ref"
    print(
        "release version invariant satisfied: "
        f"Cargo={version}, bundle=Cargo default, binary={version}, tag={tag_summary}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
