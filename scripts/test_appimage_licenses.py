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

    def test_gpl_spelled_without_a_separator_is_still_gpl(self):
        # Debian `License:` is free text; GPLv2 and GPL2 both occur in real files.
        for label in ('GPLv2', 'GPLv3', 'GPL2', 'gpl2', 'GPL',
                      'GNU General Public License v3'):
            with self.subTest(label=label):
                result = licenses.classify_labels([label])
                self.assertEqual(result['verdict'], 'ruling-required')
                self.assertIn(licenses.KIND_GPL, result['kinds'])

    def test_agpl_and_sspl_spelled_without_a_separator_stay_forbidden(self):
        for label in ('AGPL3', 'AGPLv3', 'AGPL', 'SSPL', 'SSPLv1',
                      'GNU Affero General Public License'):
            with self.subTest(label=label):
                self.assertEqual(licenses.classify_labels([label])['verdict'], 'forbidden')

    def test_lgpl_without_a_hyphen_keeps_its_notice_obligation(self):
        for label in ('LGPLv2.1', 'LGPL', 'GNU Lesser General Public License'):
            with self.subTest(label=label):
                result = licenses.classify_labels([label])
                self.assertEqual(result['verdict'], 'pass')
                self.assertTrue(result['requires_lgpl_notice'])

    def test_conjunctive_gpl_term_is_not_swallowed_by_a_leading_lgpl(self):
        # Debian uses `and` for "different parts under each license", so a GPL
        # term binds however the maintainer ordered the words.
        for label in ('LGPL-2.1+ and GPL-3+', 'GPL-3+ and LGPL-2.1+',
                      'Apache-2.0 and GPL-2'):
            with self.subTest(label=label):
                self.assertEqual(licenses.classify_labels([label])['verdict'],
                                 'ruling-required')

    def test_conjunctive_lgpl_term_still_requires_a_notice(self):
        for label in ('MIT and LGPL-2.1+', 'Apache-2.0 and LGPL-2.1+'):
            with self.subTest(label=label):
                result = licenses.classify_labels([label])
                self.assertEqual(result['verdict'], 'pass')
                self.assertTrue(result['requires_lgpl_notice'])

    def test_disjunctive_label_may_take_its_permissive_branch(self):
        # Same rule as docs/licensing.md section 5 for dual-licensed packages.
        self.assertEqual(licenses.classify_labels(
            ['LGPL-2.1 or GPL-2 at your option'])['verdict'], 'pass')
        self.assertEqual(licenses.classify_labels(['GPL-2+ or Artistic'])['verdict'], 'pass')

    def test_a_name_merely_containing_gpl_is_not_a_gpl_label(self):
        self.assertEqual(licenses.classify_label('libgpl-ish'), licenses.KIND_OTHER)


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


class RulingTests(unittest.TestCase):
    def setUp(self):
        self.saved = dict(licenses.RULINGS)
        licenses.RULINGS.clear()

    def tearDown(self):
        licenses.RULINGS.clear()
        licenses.RULINGS.update(self.saved)

    def test_a_ruling_also_answers_a_package_with_no_license_field(self):
        # Most pre-DEP-5 copyright files are readable prose. Refusing to let a
        # ruling cover them left twenty-seven MIT libraries permanently blocked.
        self.assertEqual(licenses.verdict_for_package('libxau6', [])['verdict'],
                         'unidentified')
        licenses.RULINGS['libxau6'] = 'X Consortium MIT grant, read from the copyright file'
        decided = licenses.verdict_for_package('libxau6', [])
        self.assertTrue(decided['permitted'])
        self.assertIn('MIT', decided['reason'])

    def test_a_ruling_cannot_permit_a_forbidden_license(self):
        licenses.RULINGS['libwmf-0.2-7'] = 'we would like to ship it'
        decided = licenses.verdict_for_package('libwmf-0.2-7', ['AGPL-3 with Font exception'])
        self.assertFalse(decided['permitted'])
        self.assertEqual(decided['verdict'], 'forbidden')

    def test_a_ruling_is_recorded_per_package_not_per_architecture(self):
        licenses.RULINGS['libgtk-3-0t64'] = 'GPL-3+ covers debian/* only'
        self.assertTrue(
            licenses.verdict_for_package('libgtk-3-0t64:amd64', ['GPL-3+'])['permitted'])
        self.assertTrue(
            licenses.verdict_for_package('libgtk-3-0t64:arm64', ['GPL-3+'])['permitted'])

    def test_every_recorded_ruling_states_a_reason(self):
        licenses.RULINGS.clear()
        licenses.RULINGS.update(self.saved)
        for package, reason in licenses.RULINGS.items():
            with self.subTest(package=package):
                self.assertGreater(len(reason.strip()), 20, 'a ruling needs its reason')


if __name__ == '__main__':
    unittest.main()
