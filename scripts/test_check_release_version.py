#!/usr/bin/env python3
"""Focused regression tests for the release version invariant."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from unittest.mock import patch

import check_release_version as version_check


def cargo_metadata(version: str = "0.0.3", bundle_version: str | None = None) -> dict:
    """Build the relevant cargo metadata shape for one gpssim package."""
    bundle = {"name": "Anywhere-SDR"}
    if bundle_version is not None:
        bundle["version"] = bundle_version
    return {
        "packages": [
            {
                "name": "gpssim",
                "version": version,
                "metadata": {"bundle": bundle},
            }
        ]
    }


class ReleaseVersionInvariantTests(unittest.TestCase):
    """Exercise successful and rejected release version combinations."""

    def test_matching_metadata_binary_and_tag_pass(self) -> None:
        metadata = cargo_metadata()

        cargo_version = version_check.package_version(metadata)
        executable_version = version_check.binary_version("gpssim 0.0.3\n")
        version_check.validate_versions(cargo_version, executable_version, "v0.0.3")

    def test_matching_metadata_binary_without_tag_passes_for_branch_validation(self) -> None:
        cargo_version = version_check.package_version(cargo_metadata())
        executable_version = version_check.binary_version("gpssim 0.0.3\n")

        version_check.validate_versions(cargo_version, executable_version, None)

    def test_explicit_bundle_version_is_rejected_even_when_matching(self) -> None:
        with self.assertRaisesRegex(
            version_check.VersionInvariantError,
            "bundle.version must be omitted",
        ):
            version_check.package_version(cargo_metadata(bundle_version="0.0.3"))

    def test_binary_version_mismatch_is_rejected(self) -> None:
        with self.assertRaisesRegex(
            version_check.VersionInvariantError,
            "does not match Cargo version",
        ):
            version_check.validate_versions("0.0.3", "0.0.2", None)

    def test_tag_version_mismatch_is_rejected(self) -> None:
        with self.assertRaisesRegex(
            version_check.VersionInvariantError,
            "does not match required tag",
        ):
            version_check.validate_versions("0.0.3", "0.0.3", "v0.0.2")

    def test_only_github_tag_refs_enable_tag_validation(self) -> None:
        self.assertEqual(
            version_check.tag_from_github_ref("refs/tags/v0.0.3"),
            "v0.0.3",
        )
        self.assertIsNone(version_check.tag_from_github_ref("refs/heads/master"))
        self.assertIsNone(version_check.tag_from_github_ref("refs/pull/1/merge"))
        with self.assertRaisesRegex(
            version_check.VersionInvariantError,
            "empty tag name",
        ):
            version_check.tag_from_github_ref("refs/tags/")

    def test_integrated_check_uses_locked_metadata_and_built_binary(self) -> None:
        outputs = [json.dumps(cargo_metadata()), "gpssim 0.0.3\n"]
        commands: list[list[str]] = []

        def fake_run(command: list[str]) -> str:
            commands.append(command)
            return outputs[len(commands) - 1]

        binary = Path("target/release/gpssim")
        with patch.object(version_check, "run_command", side_effect=fake_run):
            result = version_check.check_release_version(binary, "v0.0.3")

        self.assertEqual(result, "0.0.3")
        self.assertEqual(
            commands,
            [
                [
                    "cargo",
                    "metadata",
                    "--locked",
                    "--no-deps",
                    "--format-version",
                    "1",
                ],
                [str(binary), "--version"],
            ],
        )


if __name__ == "__main__":
    unittest.main()
