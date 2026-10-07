"""License classification for libraries a native bundler copied into an image."""
import unittest
import unittest.mock

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


class PackageVerdictTests(unittest.TestCase):
    def test_gpl_package_passes_only_with_a_ruling(self):
        blocked = licenses.verdict_for_package('libfuse3-4', ['GPL-2', 'LGPL-2.1'])
        self.assertFalse(blocked['permitted'])
        self.assertIsNone(blocked['reason'])
        with unittest.mock.patch.dict(
                licenses.RULINGS,
                {'libfuse3-4': 'shipped lib/* is LGPL-2.1; GPL-2 covers unshipped utilities'}):
            allowed = licenses.verdict_for_package('libfuse3-4', ['GPL-2', 'LGPL-2.1'])
        self.assertTrue(allowed['permitted'])
        self.assertIn('LGPL-2.1', allowed['reason'])

    def test_forbidden_label_cannot_be_ruled_around(self):
        with unittest.mock.patch.dict(licenses.RULINGS, {'thing': 'we really want it'}):
            result = licenses.verdict_for_package('thing', ['AGPL-3'])
        self.assertFalse(result['permitted'])

    def test_lgpl_and_unidentified_need_no_ruling_and_differ(self):
        self.assertTrue(licenses.verdict_for_package('libgtk-3-0', ['LGPL-2.1'])['permitted'])
        self.assertFalse(licenses.verdict_for_package('mystery', [])['permitted'])


class NoticeTests(unittest.TestCase):
    def test_notice_lists_every_library_with_its_source_reference(self):
        text = licenses.render_bundled_libraries([
            {'path': 'usr/lib/libgtk-3.so.0', 'package': 'libgtk-3-0t64',
             'version': '3.24.49-1', 'source_package': 'gtk+3.0',
             'source_version': '3.24.49-1', 'labels': ['LGPL-2.1']},
        ])
        self.assertIn('usr/lib/libgtk-3.so.0', text)
        self.assertIn('libgtk-3-0t64 3.24.49-1', text)
        self.assertIn('LGPL-2.1', text)
        self.assertIn(licenses.source_reference('gtk+3.0', '3.24.49-1'), text)
        self.assertIn('LGPL-2.1 text', text)

    def test_notice_refuses_to_render_an_empty_inventory(self):
        with self.assertRaisesRegex(ValueError, 'no bundled libraries'):
            licenses.render_bundled_libraries([])

    def test_library_without_a_source_renders_instead_of_raising(self):
        # Pre-flight ruling: the gate, not the renderer, is the enforcement point,
        # so an unowned library must still appear in the notice by name.
        text = licenses.render_bundled_libraries([
            {'path': 'usr/lib/liborphan.so.2', 'package': '', 'version': '',
             'source_package': '', 'source_version': '', 'labels': []},
        ])
        self.assertIn('usr/lib/liborphan.so.2', text)
        self.assertIn('UNIDENTIFIED', text)
        self.assertIn('UNAVAILABLE', text)
        self.assertIn('SOURCE-OFFER.txt', text)

    def test_source_reference_still_refuses_incomplete_input(self):
        with self.assertRaisesRegex(ValueError, 'package and a version'):
            licenses.source_reference('', '')


if __name__ == '__main__':
    unittest.main()
