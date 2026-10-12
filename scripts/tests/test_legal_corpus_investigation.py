"""Check conditional investigation profile data without HTTP or date calculation."""
import copy
import importlib.util
import inspect
import json
from pathlib import Path
import unittest
from uuid import UUID


CASE_ID = '11111111-1111-4111-8111-111111111111'
SOURCE_ID = '22222222-2222-4222-8222-222222222222'
CNPP = 'https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf'
AR630 = 'https://www2.scjn.gob.mx/juridica/engroses/1/2022/2/2_305479_6649_firmado.pdf'


def calendar_fixture():
    source = {'id': SOURCE_ID, 'title': 'Controlled calendar and closing declaration',
              'issuer': 'Fixture authority', 'official_url': 'https://example.org/fixture',
              'published_on': None, 'consulted_on': '2026-10-11',
              'locator': 'Synthetic test declaration, not an actual judicial calendar'}
    return {
        'scope': {'title': 'Controlled monthly examples', 'jurisdiction': 'federal',
                  'entity_codes': ['01'], 'authority': 'Fixture authority',
                  'organ': 'Fixture organ', 'territory': 'Controlled scope',
                  'use_description': 'Synthetic arithmetic only'},
        'coverage': {'from': '2026-01-01', 'through': '2028-12-31'},
        'sources': [source],
        'weekly_pattern': [{'weekday': day, 'classification': 'countable',
                            'source_ids': [SOURCE_ID],
                            'explanation': 'Declared countable in controlled example'}
                           for day in range(1, 8)],
        'exceptions': [],
    }


def completion_fixture():
    return {'kind': 'civil_cutoff', 'time': '07:12:34', 'offset_seconds': 19800,
            'from': '2026-01-01', 'through': '2028-12-31',
            'channel': 'Expressly declared fixture closing rule',
            'reference_id': SOURCE_ID}


def example_key(example):
    anchor = example['anchor']
    return (anchor['year'], anchor['month'], anchor['day'], example['ordered_quantity'])


class InvestigationProfileData(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path(__file__).resolve().parents[1] / 'legal_corpus_investigation.py'
        spec = importlib.util.spec_from_file_location('legal_corpus_investigation', path)
        cls.helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.helper)

    def build(self, kind='initial', maximum=2, completion=None):
        return self.helper.investigation_profile(
            CASE_ID, kind, calendar_fixture(),
            completion_fixture() if completion is None else completion,
            maximum=maximum,
        )

    def test_both_families_use_exact_qualified_ordered_month_contract(self):
        for kind in ['initial', 'extension']:
            for maximum in [2, 6]:
                with self.subTest(kind=kind, maximum=maximum):
                    profile = self.build(kind, maximum)
                    self.assertEqual(profile['scope'], {'kind': 'case', 'case_id': CASE_ID})
                    self.assertEqual(profile['trigger'], {
                        'kind': 'qualified', 'family': 'resolution',
                        'purpose': 'ordered_period_start',
                    })
                    self.assertEqual(profile['template'], {
                        'kind': 'ordered', 'unit': {
                            'kind': 'civil_months', 'final_day': 'next_countable'},
                        'maximum': maximum,
                    })

    def test_completion_is_required_and_never_replaced_with_a_default_hour(self):
        builder = self.helper.investigation_profile
        parameter = inspect.signature(builder).parameters['completion']
        self.assertIs(parameter.default, inspect.Parameter.empty)
        with self.assertRaises(TypeError):
            builder(CASE_ID, 'initial', calendar_fixture())
        self.assertEqual(self.build()['completion'], completion_fixture())
        candidate_only = {'kind': 'civil_candidate_only'}
        self.assertEqual(self.build(completion=candidate_only)['completion'], candidate_only)

    def test_completion_reference_is_supplied_evidence_not_a_new_invented_source(self):
        profile = self.build()
        references = {item['id']: item for item in profile['references']}
        self.assertEqual(references[SOURCE_ID], calendar_fixture()['sources'][0])
        unknown = completion_fixture()
        unknown['reference_id'] = '33333333-3333-4333-8333-333333333333'
        with self.assertRaises(ValueError):
            self.helper.investigation_profile(CASE_ID, 'initial', calendar_fixture(), unknown)

    def test_examples_use_separate_synthetic_calendar_and_preserve_actual_material(self):
        calendar, completion = calendar_fixture(), completion_fixture()
        calendar['coverage']['through'] = '2026-12-31'
        before = copy.deepcopy((calendar, completion))
        profile = self.helper.investigation_profile(CASE_ID, 'initial', calendar, completion)
        self.assertEqual((calendar, completion), before)
        self.assertTrue(profile['examples'])
        for example in profile['examples']:
            example_calendar = example['calendar']
            self.assertEqual(example_calendar['coverage'], {
                'from': '2026-01-01', 'through': '2028-12-31'})
            self.assertIn('synthetic', example_calendar['scope']['use_description'].lower())
            self.assertNotEqual(example_calendar['sources'][0]['id'], SOURCE_ID)
            self.assertTrue(example_calendar['scope']['entity_codes'])
            self.assertEqual({row['weekday'] for row in example_calendar['weekly_pattern']},
                             set(range(1, 8)))
            self.assertEqual({row['classification']
                              for row in example_calendar['weekly_pattern']}, {'countable'})
        profile['completion']['time'] = '01:02:03'
        profile['examples'][0]['calendar']['coverage']['through'] = '2029-12-31'
        self.assertEqual((calendar, completion), before)
        self.assertEqual(profile['examples'][1]['calendar']['coverage']['through'], '2028-12-31')

    def test_sources_are_bounded_primary_references_with_valid_links(self):
        for kind in ['initial', 'extension']:
            with self.subTest(kind=kind):
                profile = self.build(kind)
                references = {item['id']: item for item in profile['references']}
                self.assertLessEqual(len(references), 4)
                self.assertIn(CNPP, {item['official_url'] for item in references.values()})
                self.assertIn(AR630, {item['official_url'] for item in references.values()})
                for identifier in references:
                    UUID(identifier)
                for group in ['conditions', 'examples']:
                    self.assertTrue(1 <= len(profile[group]) <= 16)
                    identifiers = [item['id'] for item in profile[group]]
                    self.assertEqual(len(identifiers), len(set(identifiers)))
                    for item in profile[group]:
                        UUID(item['id'])
                        self.assertTrue(item['reference_ids'])
                        self.assertTrue(set(item['reference_ids']) <= references.keys())
                json.dumps(profile, ensure_ascii=False).encode('ascii')

    def test_initial_profile_requires_express_method_actual_duration_and_maximum_category(self):
        profile = self.build()
        statements = ' '.join(item['statement'] for item in profile['conditions']).lower()
        for phrase in ['express', 'resolution', 'ordered start', 'date-to-date',
                       'actual granted quantity', 'maximum category']:
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, statements)

    def test_extension_requires_grant_prior_periods_and_qualified_accumulated_limit(self):
        profile = self.build('extension')
        statements = ' '.join(item['statement'] for item in profile['conditions']).lower()
        for phrase in ['express extension grant', 'original period', 'previous extensions',
                       'accumulated total', 'qualified']:
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, statements)
        self.assertIn('not computed from history', profile['description'].lower())

    def test_examples_contain_no_final_date_or_invented_time_as_input(self):
        for kind in ['initial', 'extension']:
            profile = self.build(kind)
            self.assertEqual(set(profile), {'title', 'description', 'scope', 'references',
                                           'trigger', 'template', 'completion',
                                           'conditions', 'examples'})
            for example in profile['examples']:
                self.assertEqual(set(example), {'id', 'anchor', 'ordered_quantity',
                                                'calendar', 'expected',
                                                'reference_ids', 'locator'})
                self.assertEqual(set(example['anchor']), {
                    'precision', 'year', 'month', 'day', 'offset_seconds'})
                self.assertEqual(example['anchor']['precision'], 'date')
                self.assertIsNone(example['anchor']['offset_seconds'])

    def test_month_end_and_leap_examples_have_manually_specified_results(self):
        examples = {example_key(row): row['expected'] for row in self.build()['examples']}
        for key, expected in [((2027, 1, 12, 1), '2027-02-12'),
                              ((2027, 1, 31, 2), '2027-03-31'),
                              ((2028, 1, 29, 1), '2028-02-29')]:
            with self.subTest(key=key):
                self.assertEqual(examples[key], {'kind': 'arithmetic', 'outcome': {
                    'kind': 'civil_candidate', 'date': expected}})
        self.assertEqual(examples[(2027, 1, 31, 1)], {
            'kind': 'arithmetic', 'outcome': {'kind': 'blocked', 'block': {
                'kind': 'missing_homologous_day', 'year': 2027, 'month': 2,
                'requested_day': 31}}})

    def test_extension_positive_uses_the_previous_limit_as_its_declared_anchor(self):
        for maximum, key, expected in [
                (2, (2027, 2, 12, 1), '2027-03-12'),
                (6, (2027, 4, 12, 3), '2027-07-12')]:
            with self.subTest(maximum=maximum):
                examples = {example_key(row): row['expected']
                            for row in self.build('extension', maximum)['examples']}
                self.assertEqual(examples[key], {'kind': 'arithmetic', 'outcome': {
                    'kind': 'civil_candidate', 'date': expected}})

    def test_absent_quantity_and_excess_are_distinct_from_arithmetic_candidates(self):
        for maximum in [2, 6]:
            with self.subTest(maximum=maximum):
                examples = self.build(maximum=maximum)['examples']
                missing = [row for row in examples if row['ordered_quantity'] is None]
                excess = [row for row in examples if row['ordered_quantity'] == maximum + 1]
                self.assertEqual(len(missing), 1)
                self.assertEqual(len(excess), 1)
                self.assertEqual(missing[0]['expected'], {'kind': 'rule_blocked',
                    'block': {'kind': 'missing_ordered_quantity'}})
                self.assertEqual(excess[0]['expected'], {'kind': 'rule_blocked', 'block': {
                    'kind': 'ordered_quantity_exceeds_maximum',
                    'maximum': maximum, 'supplied': maximum + 1}})

    def test_invalid_kind_maximum_or_missing_material_is_not_silently_defaulted(self):
        builder = self.helper.investigation_profile
        for kind in ['', 'oral', 'requested_extension']:
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                builder(CASE_ID, kind, calendar_fixture(), completion_fixture())
        for maximum in [None, 0, 1, 3, 7, True, '2']:
            with self.subTest(maximum=maximum), self.assertRaises(ValueError):
                builder(CASE_ID, 'initial', calendar_fixture(), completion_fixture(), maximum)
        with self.assertRaises(ValueError):
            builder(CASE_ID, 'initial', None, completion_fixture())
        with self.assertRaises(ValueError):
            builder(CASE_ID, 'initial', calendar_fixture(), None)


if __name__ == '__main__':
    unittest.main()
