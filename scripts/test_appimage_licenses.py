"""License classification for libraries a native bundler copied into an image."""
import unittest

import appimage_licenses as licenses


class ClassifyTests(unittest.TestCase):
    def test_lgpl_is_permitted_and_is_not_read_as_gpl(self):
        for label in ('LGPL-2.1', 'LGPL-2.1+', 'LGPL-2', 'lgpl-3'):
            with self.subTest(label=label):
                result = licenses.classify_labels([label])
                self.assertEqual(result['verdict'], 'pass')
                self.assertEqual(result['kinds'], [licenses.KIND_LGPL])
                self.assertTrue(result['requires_lgpl_notice'])

    def test_gpl_requires_a_recorded_ruling(self):
        for label in ('GPL-2', 'GPL-2+', 'GPL-3+ with autoconf exception'):
            with self.subTest(label=label):
                result = licenses.classify_labels([label])
                self.assertEqual(result['verdict'], 'ruling-required')
                self.assertIn(licenses.KIND_GPL, result['kinds'])

    def test_agpl_and_sspl_are_forbidden_outright(self):
        for label in ('AGPL-3', 'AGPL-3+', 'SSPL-1.0'):
            with self.subTest(label=label):
                self.assertEqual(licenses.classify_labels([label])['verdict'], 'forbidden')

    def test_mixed_gpl_and_lgpl_requires_a_ruling(self):
        # libfuse: `Files: *` is GPL-2 while the shipped `lib/*` is LGPL-2.1.
        result = licenses.classify_labels(['GPL-2', 'LGPL-2.1'])
        self.assertEqual(result['verdict'], 'ruling-required')
        self.assertTrue(result['requires_lgpl_notice'])

    def test_unrecognized_but_present_labels_pass_without_notice(self):
        result = licenses.classify_labels(['public-domain', 'Expat', 'BSD-3-clause'])
        self.assertEqual(result['verdict'], 'pass')
        self.assertEqual(result['kinds'], [licenses.KIND_OTHER])
        self.assertFalse(result['requires_lgpl_notice'])

    def test_no_labels_is_unidentified_not_pass(self):
        self.assertEqual(licenses.classify_labels([])['verdict'], 'unidentified')


if __name__ == '__main__':
    unittest.main()
