"""Check the finite resource examples documented in docs/legal-corpus-resources.md."""
import copy
from datetime import datetime, timedelta, timezone
import importlib.util
import inspect
from pathlib import Path
import unittest
from uuid import UUID

KINDS = {'oral', 'written', 'control_order', 'control_sentence', 'withdrawal', 'trial_sentence'}
DAILY = {'written': (2, '2026-10-16'), 'control_order': (3, '2026-10-19'),
         'control_sentence': (5, '2026-10-21'), 'withdrawal': (3, '2026-10-19'),
         'trial_sentence': (10, '2026-10-28')}
CNPP = 'https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf'
ADR = 'https://sjf2.scjn.gob.mx/detalle/ejecutoria/34255'
CASE = '11111111-1111-4111-8111-111111111111'


def calendar():
    reference = '22222222-2222-4222-8222-222222222222'
    return {
        'scope': {'title': 'Controlled weekday calendar', 'jurisdiction': 'local',
                  'entity_codes': ['09'], 'authority': 'Fixture authority',
                  'organ': 'Fixture organ', 'territory': 'CDMX',
                  'use_description': 'Controlled weekdays, not an official calendar'},
        'coverage': {'from': '2026-10-01', 'through': '2026-10-31'},
        'sources': [{'id': reference, 'title': 'Controlled calendar', 'issuer': 'Fixture',
                     'official_url': 'https://example.org/calendar', 'published_on': None,
                     'consulted_on': '2026-10-11', 'locator': 'Weekdays only'}],
        'weekly_pattern': [{'weekday': day, 'classification': 'countable' if day < 6 else 'excluded',
                            'source_ids': [reference], 'explanation': 'Controlled weekday pattern'}
                           for day in range(1, 8)],
        'exceptions': [],
    }


class LegalCorpusResources(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path(__file__).resolve().parents[1] / 'legal_corpus_resources.py'
        spec = importlib.util.spec_from_file_location('legal_corpus_resources', path)
        cls.builder = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.builder)

    def profile(self, kind, selected_calendar=None, completion=None):
        policy = completion if completion is not None else {
            'kind': 'arithmetic_instant' if kind == 'oral' else 'civil_candidate_only'}
        return self.builder.resource_profile(CASE, kind, selected_calendar or calendar(), policy)

    def test_catalog_contains_exactly_the_six_documented_resources(self):
        self.assertEqual(set(self.builder.RESOURCE_KINDS), KINDS)
        self.assertEqual(len(self.builder.RESOURCE_KINDS), 6)
        with self.assertRaises(ValueError):
            self.profile('appeal')

    def test_daily_examples_use_declared_effects_and_the_independent_civil_oracles(self):
        for kind, (quantity, expected) in DAILY.items():
            with self.subTest(kind=kind):
                selected = calendar()
                value = self.profile(kind, selected)
                self.assertEqual(value['scope'], {'kind': 'case', 'case_id': CASE})
                self.assertEqual(value['trigger'], {
                    'kind': 'source_field', 'field': 'notification_stated_effect_at'})
                self.assertEqual(value['template'], {'kind': 'fixed', 'rule': {
                    'kind': 'days', 'quantity': quantity, 'inclusion': 'on_anchor',
                    'basis': 'calendar_countable', 'final_day': 'preserve'}})
                example = value['examples'][0]
                self.assertEqual(example['anchor'], {'precision': 'date', 'year': 2026,
                    'month': 10, 'day': 15, 'offset_seconds': None})
                self.assertIsNone(example['ordered_quantity'])
                self.assertEqual(example['calendar'], selected)
                self.assertEqual(example['expected'], {'kind': 'arithmetic',
                    'outcome': {'kind': 'civil_candidate', 'date': expected}})
                inputs = copy.deepcopy(example)
                del inputs['expected']
                self.assertNotIn(expected, repr(inputs))
                self.assertNotIn('due_at', value)

    def test_oral_example_ends_at_the_exact_declared_hearing_time(self):
        value = self.profile('oral')
        self.assertEqual(value['trigger'], {
            'kind': 'qualified', 'purpose': 'hearing_end', 'family': 'hearing_result'})
        self.assertEqual(value['template'], {'kind': 'fixed', 'rule': {'kind': 'at_anchor'}})
        self.assertEqual(value['completion'], {'kind': 'arithmetic_instant'})
        example = value['examples'][0]
        self.assertEqual(example['anchor'], {'precision': 'second', 'year': 2026, 'month': 10,
            'day': 9, 'hour': 10, 'minute': 50, 'second': 0, 'offset_seconds': -21600})
        self.assertIsNone(example['calendar'])
        self.assertIsNone(example['ordered_quantity'])
        expected = datetime(2026, 10, 9, 10, 50, tzinfo=timezone(timedelta(hours=-6)))
        self.assertEqual(example['expected'], {'kind': 'arithmetic', 'outcome': {
            'kind': 'instant_candidate', 'instant': {'unix_seconds': int(expected.timestamp()),
                'nanosecond': 0, 'offset_seconds': -21600}}})

    def test_cnpp_supports_every_profile_and_trial_alone_uses_the_scjn_decision(self):
        for kind in KINDS:
            with self.subTest(kind=kind):
                value = self.profile(kind)
                references = {row['id']: row for row in value['references']}
                self.assertEqual({row['official_url'] for row in references.values()},
                                 {CNPP, ADR} if kind == 'trial_sentence' else {CNPP})
                cnpp = next(row for row in references.values() if row['official_url'] == CNPP)
                self.assertIn('2025', cnpp['title'] + cnpp['locator'])
                self.assertIn('466' if kind in ['oral', 'written'] else '471', cnpp['locator'])
                for row in value['conditions'] + value['examples']:
                    self.assertTrue(row['reference_ids'])
                    self.assertLessEqual(set(row['reference_ids']), references.keys())

    def test_conditions_keep_the_distinct_resolution_authority_and_notification_modes(self):
        expected = {
            'oral': ['tramite', 'sin sustanciacion', 'audiencia', 'fin'],
            'written': ['fuera de audiencia', 'notificacion personal', 'instalaciones'],
            'control_order': ['juez de control', 'anticipo de prueba', 'notificacion personal', 'audiencia'],
            'control_sentence': ['juez de control', 'sentencia definitiva', 'abreviado', 'notificacion personal'],
            'withdrawal': ['desistimiento', 'accion penal', 'ministerio publico', 'tribunal de enjuiciamiento'],
            'trial_sentence': ['tribunal de enjuiciamiento', 'lectura', 'explicacion', 'version escrita',
                               'disponible', 'presente', 'notificacion personal'],
        }
        for kind, phrases in expected.items():
            with self.subTest(kind=kind):
                statements = ' '.join(row['statement'] for row in self.profile(kind)['conditions']).lower()
                for phrase in phrases + ['cdmx', 'suspension', 'reposicion', 'abreviacion']:
                    self.assertIn(phrase, statements)
                self.assertNotIn('revision humana aprobada', statements)

    def test_completion_is_required_without_an_invented_cutoff_and_inputs_are_copied(self):
        signature = inspect.signature(self.builder.resource_profile)
        self.assertIs(signature.parameters['completion'].default, inspect.Parameter.empty)
        with self.assertRaises(TypeError):
            self.builder.resource_profile(CASE, 'written', calendar())
        selected, policy = calendar(), {'kind': 'civil_candidate_only'}
        original = copy.deepcopy(selected)
        first = self.profile('written', selected, policy)
        second = self.profile('written', selected, policy)
        selected['exceptions'].append({'unexpected': True})
        policy['kind'] = 'changed'
        self.assertEqual(first['completion'], {'kind': 'civil_candidate_only'})
        self.assertEqual(first['examples'][0]['calendar'], original)
        own_ids = lambda value: {row['id'] for key in ['references', 'conditions', 'examples']
                                 for row in value[key]}
        self.assertTrue(own_ids(first).isdisjoint(own_ids(second)))
        for value in own_ids(first) | own_ids(second):
            self.assertNotEqual(UUID(value).int, 0)

    def test_injected_weekly_completion_preserves_its_matching_reference(self):
        reference = calendar()['sources'][0]
        policy = {'kind': 'weekly_civil_cutoff',
                  'times': ['18:00:00'] * 4 + ['15:00:00', None, None],
                  'offset_seconds': -21600, 'from': '2026-10-01', 'through': '2026-10-31',
                  'channel': 'Explicit test channel', 'reference_id': reference['id']}
        expected_policy, expected_reference = copy.deepcopy(policy), copy.deepcopy(reference)
        value = self.builder.resource_profile(CASE, 'written', calendar(), policy,
                                              completion_reference=reference)
        policy['times'][0] = None
        reference['title'] = 'Changed caller data'
        self.assertEqual(value['completion'], expected_policy)
        self.assertIn(expected_reference, value['references'])
        self.assertEqual(value['examples'][0]['expected']['outcome']['date'], '2026-10-16')

    def test_a_civil_completion_cannot_reference_an_absent_or_different_source(self):
        reference = calendar()['sources'][0]
        for kind in ['civil_cutoff', 'weekly_civil_cutoff']:
            policy = {'kind': kind, 'reference_id': reference['id']}
            for supplied in [None, {**reference, 'id': CASE}]:
                with self.subTest(kind=kind, supplied=supplied), self.assertRaises(ValueError):
                    self.builder.resource_profile(CASE, 'written', calendar(), policy,
                                                  completion_reference=supplied)


if __name__ == '__main__':
    unittest.main()
