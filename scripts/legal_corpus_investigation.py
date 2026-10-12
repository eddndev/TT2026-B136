"""Build conditional monthly investigation profiles without transport or date math."""
from copy import deepcopy
from uuid import uuid4


CNPP_URL = 'https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf'
AR630_URL = 'https://www2.scjn.gob.mx/juridica/engroses/1/2022/2/2_305479_6649_firmado.pdf'
EXAMPLES_URL = (
    'https://github.com/eddndev/TT2026-B136/blob/main/docs/legal-corpus-investigation.md'
)


def _reference(title, issuer, url, locator, published_on=None):
    return {'id': str(uuid4()), 'title': title, 'issuer': issuer,
            'official_url': url, 'published_on': published_on,
            'consulted_on': '2026-10-11', 'locator': locator}


def _example_calendar(source):
    return {
        'scope': {'title': 'Synthetic monthly arithmetic examples',
                  'jurisdiction': 'federal', 'entity_codes': ['01'],
                  'authority': 'Controlled fixture', 'organ': 'Controlled fixture',
                  'territory': 'Controlled examples only',
                  'use_description': 'Synthetic examples, not an official judicial calendar'},
        'coverage': {'from': '2026-01-01', 'through': '2028-12-31'},
        'sources': [deepcopy(source)],
        'weekly_pattern': [
            {'weekday': day, 'classification': 'countable', 'source_ids': [source['id']],
             'explanation': 'Synthetic countable day for isolated monthly arithmetic'}
            for day in range(1, 8)
        ],
        'exceptions': [],
    }


def _examples(kind, maximum, calendar, reference_ids):
    # These expected dates are specified independently in the versioned corpus note.
    candidates = [
        ((2027, 1, 12), 1, '2027-02-12', 'Express one-month order'),
        ((2027, 1, 31), 2, '2027-03-31', 'Direct two-month advance with a homologue'),
        ((2028, 1, 29), 1, '2028-02-29', 'Leap-year homologue'),
    ]
    if kind == 'extension':
        candidates.append({
            2: ((2027, 2, 12), 1, '2027-03-12', 'One original month plus one granted month'),
            6: ((2027, 4, 12), 3, '2027-07-12', 'Three original months plus three granted months'),
        }[maximum])
    rows = []

    def append(anchor, quantity, expected, locator):
        year, month, day = anchor
        rows.append({
            'id': str(uuid4()),
            'anchor': {'precision': 'date', 'year': year, 'month': month,
                       'day': day, 'offset_seconds': None},
            'ordered_quantity': quantity, 'calendar': deepcopy(calendar),
            'expected': expected, 'reference_ids': list(reference_ids), 'locator': locator,
        })

    for anchor, quantity, expected, locator in candidates:
        append(anchor, quantity, {'kind': 'arithmetic', 'outcome': {
            'kind': 'civil_candidate', 'date': expected}}, locator)
    append((2027, 1, 31), 1, {'kind': 'arithmetic', 'outcome': {
        'kind': 'blocked', 'block': {'kind': 'missing_homologous_day',
                                   'year': 2027, 'month': 2, 'requested_day': 31}}},
        'No express policy for an absent homologue')
    append((2027, 1, 12), None, {'kind': 'rule_blocked',
        'block': {'kind': 'missing_ordered_quantity'}}, 'Granted duration is absent')
    supplied = {2: 3, 6: 7}[maximum]
    append((2027, 1, 12), supplied, {'kind': 'rule_blocked', 'block': {
        'kind': 'ordered_quantity_exceeds_maximum', 'maximum': maximum,
        'supplied': supplied}}, 'Supplied quantity exceeds the selected maximum')
    return rows


def investigation_profile(case_id, kind, calendar, completion, maximum=2):
    """Return an API definition; operational calendar and closing evidence stay explicit.

    Embedded examples use a separate synthetic calendar. Extension applicability
    requires source-qualified accumulation, not automatic historical arithmetic.
    """
    if kind not in ('initial', 'extension'):
        raise ValueError('unknown investigation profile kind')
    if type(maximum) is not int or maximum not in (2, 6):
        raise ValueError('maximum must be the qualified two- or six-month category')
    if not isinstance(calendar, dict) or not isinstance(calendar.get('sources'), list):
        raise ValueError('an explicit operational calendar is required')
    if not isinstance(completion, dict) or completion.get('kind') not in (
            'civil_cutoff', 'civil_candidate_only'):
        raise ValueError('an explicit civil completion policy is required')
    cnpp = _reference('Codigo Nacional de Procedimientos Penales',
                      'Camara de Diputados', CNPP_URL,
                      'Articles 94, 321 and 322; pages 24-25 and 96; reform 2025-11-28')
    precedent = _reference('Amparo en revision 630/2022', 'SCJN Primera Sala',
                           AR630_URL, 'Paragraphs 37-42 and 50-51; printed pages 18-22',
                           '2023-09-20')
    example_source = _reference('Synthetic monthly examples', 'Versioned test corpus',
                                EXAMPLES_URL,
                                'Controlled monthly examples; not official future holidays')
    references = [cnpp, precedent, example_source]
    if completion['kind'] == 'civil_cutoff':
        matching = [source for source in calendar['sources']
                    if source.get('id') == completion.get('reference_id')]
        if len(matching) != 1:
            raise ValueError('closing reference must identify one supplied calendar source')
        references.append(deepcopy(matching[0]))
    statements = [
        'The resolution expressly identifies the ordered start and a date-to-date method, '
        'retaining the same numbered day without adding or subtracting a day. '
        'No missing-homologue policy is inferred.',
        'The actual granted quantity is expressly stated in civil months, and the '
        'maximum category is qualified from the applicable penalty range as '
        + str(maximum) + ' months. The maximum does not supply a missing duration.',
        'The actual scope, calendar and closing rule are expressly qualified. '
        'Synthetic examples do not establish official holidays or an operational closing hour.',
        'No unresolved incident, suspension or early closure prevents applicability.',
    ]
    if kind == 'extension':
        statements.extend([
            'An express extension grant states the granted duration and ordered start. '
            'A request alone does not establish a grant.',
            'The original period and previous extensions are identified, and the '
            'accumulated total in the same unit is expressly qualified within the '
            + str(maximum) + '-month maximum measured from the original start.',
        ])
    description = (
        'Conditional express judicial order using civil months and date-to-date counting. '
        'The supplied closing policy remains explicit. Embedded examples are isolated '
        'synthetic arithmetic, not an official future calendar.'
    )
    if kind == 'extension':
        description += ' Accumulation is source-qualified, not computed from history.'
    return {
        'title': 'Investigation ' + kind + ': express monthly order',
        'description': description, 'scope': {'kind': 'case', 'case_id': case_id},
        'references': references,
        'trigger': {'kind': 'qualified', 'family': 'resolution',
                    'purpose': 'ordered_period_start'},
        'template': {'kind': 'ordered',
                     'unit': {'kind': 'civil_months', 'final_day': 'next_countable'},
                     'maximum': maximum},
        'completion': deepcopy(completion),
        'conditions': [{'id': str(uuid4()), 'statement': statement,
                        'reference_ids': [cnpp['id'], precedent['id']]}
                       for statement in statements],
        'examples': _examples(kind, maximum, _example_calendar(example_source),
                              [cnpp['id'], example_source['id']]),
    }
