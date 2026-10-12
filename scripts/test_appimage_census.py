"""Reproducible verdict census over a recorded AppImage payload inventory."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import appimage_census as census


INVENTORY = {
    'artifact': 'FontCarver.AppImage',
    'packages': [
        {'package': 'libpng16-16t64', 'labels': ['GPL-2+', 'Expat']},
        {'package': 'libpixman-1-0', 'labels': ['Expat']},
        {'package': 'libwmf-0.2-7', 'labels': ['AGPL-3 with Font exception']},
        {'package': 'libxdmcp6', 'labels': []},
        {'package': 'libglib2.0-0t64', 'labels': ['LGPL-2.1+']},
    ],
}


class CensusTests(unittest.TestCase):
    def test_every_package_is_counted_under_one_verdict(self):
        report = census.census(INVENTORY)
        self.assertEqual(report['counts'],
                         {'pass': 2, 'ruling-required': 1, 'unidentified': 1, 'forbidden': 1})
        self.assertEqual(sum(report['counts'].values()), len(report['packages']))

    def test_rows_carry_the_package_and_its_labels(self):
        rows = {row['package']: row for row in census.census(INVENTORY)['packages']}
        self.assertEqual(rows['libwmf-0.2-7']['verdict'], 'forbidden')
        self.assertEqual(rows['libxdmcp6']['labels'], [])
        self.assertEqual(rows['libpng16-16t64']['verdict'], 'ruling-required')

    def test_the_report_names_the_inventory_it_came_from(self):
        report = census.census(INVENTORY)
        self.assertEqual(report['artifact'], 'FontCarver.AppImage')
        self.assertEqual(report['schema_version'], 1)

    def test_an_inventory_without_packages_is_rejected(self):
        # An empty census must never read as "nothing to rule on".
        with self.assertRaisesRegex(ValueError, 'no packages'):
            census.census({'artifact': 'x', 'packages': []})

    def test_the_command_writes_json_and_reports_blockers_in_its_status(self):
        with tempfile.TemporaryDirectory() as work:
            source = Path(work) / 'inventory.json'
            source.write_text(json.dumps(INVENTORY), encoding='utf-8')
            output = Path(work) / 'census.json'
            self.assertEqual(census.main(['--inventory', str(source), '--output', str(output)]), 1)
            written = json.loads(output.read_text(encoding='utf-8'))
            self.assertEqual(written['counts']['forbidden'], 1)

            clean = Path(work) / 'clean.json'
            clean.write_text(json.dumps(
                {'artifact': 'x', 'packages': [{'package': 'a', 'labels': ['Expat']}]}),
                encoding='utf-8')
            self.assertEqual(census.main(['--inventory', str(clean),
                                          '--output', str(Path(work) / 'ok.json')]), 0)


if __name__ == '__main__':
    unittest.main()
