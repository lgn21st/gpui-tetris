"""Artifact corruption and source mismatch must stop upload before any mutation."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import release


class ReleaseIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        archive = self.directory / "app.zip"
        archive.write_bytes(b"original archive")
        self.manifest = {
            "version": "0.0.0", "source_commit": "tested-commit", "archive": archive.name,
            "archive_sha256": release.digest(archive),
        }
        (self.directory / "build-manifest.json").write_text(json.dumps(self.manifest))
        (self.directory / "dependency-audit.json").write_text("{}")
        (self.directory / "SHA256SUMS").write_text("".join(
            f"{release.digest(p)}  {p.name}\n" for p in self.directory.iterdir()
        ))

    def test_archive_corruption_is_rejected(self):
        release.verify_candidate(self.directory)
        (self.directory / "app.zip").write_bytes(b"corrupt archive")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            release.verify_candidate(self.directory)

    def test_checksum_path_cannot_escape_candidate(self):
        (self.directory / "SHA256SUMS").write_text("abc  ../outside.zip\n")
        with self.assertRaisesRegex(ValueError, "Invalid checksum filename"):
            release.verify_candidate(self.directory)

    def test_missing_manifest_checksum_is_rejected(self):
        checksums = self.directory / "SHA256SUMS"
        checksums.write_text("\n".join(
            line for line in checksums.read_text().splitlines() if "build-manifest" not in line
        ) + "\n")
        with self.assertRaisesRegex(ValueError, "Missing or duplicated"):
            release.verify_candidate(self.directory)

    def test_unrelated_commit_cannot_be_uploaded(self):
        args = type("Args", (), {"directory": self.directory})()
        with patch.object(release, "source", return_value=({"version": "0.0.0"}, "other-commit")):
            with patch.object(release, "run") as run:
                with self.assertRaisesRegex(ValueError, "does not match"):
                    release.publish(args)
                run.assert_not_called()

    def test_failed_ci_prevents_release_creation(self):
        args = type("Args", (), {"directory": self.directory})()
        with patch.object(release, "source", return_value=({"version": "0.0.0"}, "tested-commit")):
            with patch.object(release, "run", side_effect=[
                "tested-commit\trefs/tags/v0.0.0^{}\n",
                '[{"status":"completed","conclusion":"failure"}]',
            ]) as run:
                with self.assertRaisesRegex(ValueError, "CI runs"):
                    release.publish(args)
                self.assertEqual(run.call_count, 2)

    def test_corrupt_download_never_publishes_draft(self):
        notes = self.directory / "notes.md"
        notes.write_text("Release test")
        args = type("Args", (), {
            "directory": self.directory, "notes": notes, "publish": True,
        })()

        def command(*args, **kwargs):
            if args[:2] == ("git", "ls-remote"):
                return "tested-commit\trefs/tags/v0.0.0^{}\n"
            if args[:3] == ("gh", "run", "list"):
                return '[{"status":"completed","conclusion":"success"}]'
            if args[:3] == ("gh", "release", "download"):
                destination = Path(args[args.index("--dir") + 1])
                for original in self.directory.iterdir():
                    (destination / original.name).write_bytes(b"corrupted")
            return ""

        with patch.object(release, "source", return_value=({"version": "0.0.0"}, "tested-commit")):
            with patch.object(release, "run", side_effect=command) as run:
                with self.assertRaisesRegex(ValueError, "Uploaded asset differs"):
                    release.publish(args)
                self.assertFalse(any(call.args[:3] == ("gh", "release", "edit")
                                     for call in run.call_args_list))


if __name__ == "__main__":
    unittest.main()
