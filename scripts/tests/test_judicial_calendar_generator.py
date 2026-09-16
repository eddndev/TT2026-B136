import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / 'crates/domain/tests/fixtures/generate_judicial_calendar_vectors.py'
spec = importlib.util.spec_from_file_location('calendar_vectors', PATH)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def minimum():
    return {'scope': {'title': 'x', 'jurisdiction': 'federal', 'entity_codes': ['01'],
                     'authority': 'x', 'organ': 'x', 'territory': 'x', 'use_description': 'x'},
            'coverage': {'from': '1970-01-01', 'through': '1970-01-01'}, 'sources': [],
            'weekly_pattern': [{'weekday': n, 'classification': 'unresolved',
                                'source_ids': [], 'explanation': 'x'} for n in range(1, 8)],
            'exceptions': []}


class GeneratorTests(unittest.TestCase):
    def test_independent_minimum_layout(self):
        expected = '4a43414c31' + '0000000178' + '000101'
        expected += '0000000178' * 4 + '0000000000000000' + '00'
        expected += ''.join(f'{n:02x}02000000000178' for n in range(1, 8)) + '00'
        self.assertEqual(len(bytes.fromhex(expected)), 99)
        self.assertEqual(module.encode(minimum()).hex(), expected)

    def test_maximum_size_from_field_sum(self):
        value = module.maximum()
        self.assertEqual(len(module.encode(value)), 191910)
        self.assertEqual(len(value['scope']['entity_codes']), 32)
        self.assertEqual(len(value['sources']), 16)
        self.assertEqual(len(value['exceptions']), 64)

    def test_epoch_and_supported_extremes(self):
        for day, encoded in [('0001-01-01', 'fff506c6'), ('9999-12-31', '002cc0a0')]:
            value = minimum()
            value['coverage'] = {'from': day, 'through': day}
            self.assertEqual(module.encode(value)[33:41].hex(), encoded * 2)

    def test_noncanonical_input_has_same_encoding(self):
        value = minimum()
        value['weekly_pattern'].reverse()
        value['scope']['title'] = ' x '
        value['scope']['use_description'] = '\r\nx\r\n'
        self.assertEqual(module.normalize(value), minimum())
        self.assertEqual(module.encode(value), module.encode(minimum()))

    def test_invalid_days_coverage_references_and_controls(self):
        for field, replacement in [('coverage', {'from': '1900-02-29', 'through': '1900-03-01'}),
                                   ('coverage', {'from': '2026-01-01', 'through': '2029-01-01'}),
                                   ('weekly_pattern', [])]:
            value = minimum(); value[field] = replacement
            with self.assertRaises(ValueError): module.normalize(value)
        for title in ['\tx', 'x\r', 'x\x7f', '\ud800']:
            value = minimum(); value['scope']['title'] = title
            with self.assertRaises(ValueError): module.normalize(value)
        value = minimum(); value['weekly_pattern'][0]['classification'] = 'countable'
        with self.assertRaises(ValueError): module.normalize(value)

    def test_exact_uri_profile_preserves_text(self):
        for url in ['https://EXAMPLE.invalid', 'https://a.example/?x=%2f#part?query',
                    "https://example.invalid/a!$&'()*+,;=:@/?a?b#c?d", ' https://example.invalid/ ']:
            self.assertEqual(module.official_url(url), url.strip())
        for url in ['HTTPS://example.invalid', 'https://user@example.invalid',
                    'https://example.invalid:443/', 'https://127.0.0.1/', 'https://[::1]/',
                    'https://localhost/', 'https://xn--example.invalid/', 'https://a..invalid/',
                    'https://a.1a/', 'https://example.invalid/%', 'https://example.invalid/%GG',
                    'https://example.invalid/a#b#c', 'https://example.invalid/a"b',
                    'https://example.invalid/{x}', 'https://example.invalid/[x]',
                    'https://example.invalid/a b', 'https://example.invalid/\\evil']:
            with self.subTest(url=url), self.assertRaises(ValueError): module.official_url(url)

    def test_overlapping_intervals_and_missing_references_are_rejected(self):
        value = minimum()
        value['exceptions'] = [dict(id=f'00000000-0000-0000-0000-{n:012}',
            **{'from': '1970-01-01', 'through': '1970-01-01'}, classification='unresolved',
            source_ids=[], explanation='x') for n in [1, 2]]
        with self.assertRaises(ValueError): module.normalize(value)
        value = minimum()
        value['weekly_pattern'][0]['source_ids'] = ['00000000-0000-0000-0000-000000000000']
        with self.assertRaises(ValueError): module.normalize(value)

    def test_publication_date_and_canonical_order(self):
        rows = module.vectors()
        self.assertEqual(rows[3]['hex'], rows[4]['hex'])
        self.assertNotEqual(rows[4]['hex'], rows[5]['hex'])
        value = copy.deepcopy(rows[3]['input'])
        value['sources'][0]['published_on'] = '2000-03-01'
        with self.assertRaises(ValueError): module.normalize(value)


    def test_submission_boundaries_and_layout(self):
        nil = '00000000-0000-0000-0000-000000000000'
        digest = '00' * 32
        actual = module.submission(nil, nil, nil, 'publish', 0, digest)
        self.assertEqual(actual, b'JCTX1' + bytes(86))
        self.assertEqual(len(actual), 91)
        maximal = module.submission(nil, nil, nil, 'replace', 0xfffffffe, digest, chr(0x1f642) * 1000)
        self.assertEqual(len(maximal), 4095)
        self.assertEqual(maximal[53:58].hex(), '01fffffffe')
        with self.assertRaises(ValueError): module.submission(nil, nil, nil, 'publish', 1, digest)
        with self.assertRaises(ValueError): module.submission(nil, nil, nil, 'retire', 0xffffffff, digest, 'x')


if __name__ == '__main__': unittest.main()
