"""Check disk-budget accounting and failure behavior without large artifacts."""
import argparse
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_artifacts as artifacts


class ArtifactBudgetTests(unittest.TestCase):
    def test_hard_links_count_once_and_directory_links_are_not_followed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / 'output'
            output.mkdir()
            source = output / 'file'
            source.write_bytes(b'x' * 4096)
            os.link(source, output / 'hard-link')
            external = root / 'external'
            external.mkdir()
            (external / 'large').write_bytes(b'x' * 1024 * 1024)
            (output / 'external-link').symlink_to(external, target_is_directory=True)
            size, count = artifacts.inventory(output, set())
            self.assertEqual(count, 2)  # One file inode and one link.
            self.assertLess(size, 1024 * 1024)

    def test_a_link_as_artifact_root_is_not_silently_underreported(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            link = root / 'link'
            link.symlink_to(root, target_is_directory=True)
            with self.assertRaises(ValueError):
                artifacts.inventory(link, set())

    def test_exceeded_budget_fails_without_deleting_anything(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'target').mkdir()
            artifact = root / 'target/file'
            artifact.write_bytes(b'keep')
            args = argparse.Namespace(build_dir=[], task_temp=[], build_limit_gib=0.,
                                      temp_limit_gib=1., min_free_gib=0.)
            with patch.object(artifacts, 'ROOT', root):
                report = artifacts.check(args)
            self.assertFalse(report['ok'])
            self.assertTrue(any('build artifacts' in error for error in report['errors']))
            self.assertEqual(artifact.read_bytes(), b'keep')

    def test_free_space_floor_is_checked_even_with_no_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            args = argparse.Namespace(build_dir=[], task_temp=[], build_limit_gib=20.,
                                      temp_limit_gib=1., min_free_gib=20.)
            with patch.object(artifacts, 'ROOT', Path(directory)), patch.object(
                artifacts.shutil, 'disk_usage', return_value=argparse.Namespace(free=0),
            ):
                report = artifacts.check(args)
            self.assertFalse(report['ok'])
            self.assertTrue(any('free at' in error for error in report['errors']))

    def test_nonfinite_and_negative_budgets_are_rejected(self):
        for value in ['nan', 'inf', '-1']:
            with self.assertRaises(argparse.ArgumentTypeError):
                artifacts.nonnegative(value)


if __name__ == '__main__':
    unittest.main()
