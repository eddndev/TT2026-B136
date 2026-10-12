"""Build the six controlled resource profiles in docs/legal-corpus-resources.md.

The caller supplies the calendar and completion policy. Expected civil dates
come from the documented enumeration, not from a deadline evaluator.
"""
import copy
from uuid import uuid4

RESOURCE_KINDS = ('oral', 'written', 'control_order', 'control_sentence',
                  'withdrawal', 'trial_sentence')

_RESOURCES = {
    'oral': ('Revocacion oral al fin de audiencia', None, None,
             'Arts. 465 y 466.I, p. 132', [
        'La resolucion es de mero tramite y fue dictada sin sustanciacion durante la audiencia.',
        'El destinatario recurrente esta presente y recibe notificacion personal de la resolucion en esa audiencia.',
        'La fuente identifica la audiencia y su fin declarado con fecha, hora y desfase exactos.',
        'La oportunidad oral exige actuar antes del fin de audiencia; ese instante no es un dia posterior.',
    ]),
    'written': ('Revocacion escrita fuera de audiencia', 2, '2026-10-16',
                'Arts. 465 y 466.II, pp. 132-133; arts. 82, 84 y 94, pp. 21-25', [
        'La resolucion es de mero tramite, dictada sin sustanciacion y fuera de audiencia.',
        'La notificacion personal al destinatario se practico en instalaciones del organo jurisdiccional.',
    ]),
    'control_order': ('Apelacion de auto del juez de control', 3, '2026-10-19',
                     'Arts. 467.I y 471, primer parrafo, pp. 133-134; arts. 82, 84 y 94, pp. 21-25', [
        'El juez de control nego el anticipo de prueba mediante auto; no se trata de una sentencia.',
        'La notificacion personal al destinatario se practico en audiencia, con el recurrente presente.',
    ]),
    'control_sentence': ('Apelacion de sentencia definitiva del juez de control', 5, '2026-10-21',
                        'Arts. 467.X y 471, primer parrafo, pp. 133-134; arts. 82, 84 y 94, pp. 21-25', [
        'El juez de control dicto sentencia definitiva en procedimiento abreviado.',
        'La notificacion personal al destinatario se practico en audiencia, con el recurrente presente.',
    ]),
    'withdrawal': ('Apelacion sobre desistimiento de la accion penal', 3, '2026-10-19',
                   'Art. 471, segundo parrafo, p. 134; arts. 82, 84 y 94, pp. 21-25', [
        'La resolucion versa sobre desistimiento de la accion penal del Ministerio Publico ante el tribunal de enjuiciamiento.',
        'La fuente identifica ese desistimiento de la accion penal; no es el retiro de un recurso.',
        'La notificacion personal al destinatario se practico en audiencia, con el recurrente presente.',
    ]),
    'trial_sentence': ('Apelacion de sentencia definitiva de enjuiciamiento', 10, '2026-10-28',
                       'Art. 471, segundo parrafo, pp. 134-135; arts. 82, 84, 87 y 94, pp. 21-25', [
        'El tribunal de enjuiciamiento dicto la sentencia definitiva objeto de la apelacion.',
        'La notificacion personal se practico en la audiencia de lectura y explicacion, con el destinatario recurrente presente.',
        'La version escrita de la sentencia estuvo disponible para el destinatario durante esa audiencia.',
    ]),
}


def _legal_references(kind, locator):
    references = [{
        'id': str(uuid4()),
        'title': 'Codigo Nacional de Procedimientos Penales, reforma DOF 28-11-2025',
        'issuer': 'Camara de Diputados del Congreso de la Union',
        'official_url': 'https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf',
        'published_on': '2025-11-28', 'consulted_on': '2026-10-11', 'locator': locator,
    }]
    if kind == 'trial_sentence':
        references.append({
            'id': str(uuid4()), 'title': 'Amparo directo en revision 2742/2023, registro 34255',
            'issuer': 'Suprema Corte de Justicia de la Nacion, Pleno',
            'official_url': 'https://sjf2.scjn.gob.mx/detalle/ejecutoria/34255',
            'published_on': None, 'consulted_on': '2026-10-11',
            'locator': 'Sentencia 27-01-2026; SJF, Libro 13, septiembre 2026; '
                       'antecedentes 9-11; parrafos 101-102, 116 y 146',
        })
    return references


def _completion(completion, completion_reference):
    if not isinstance(completion, dict) or not isinstance(completion.get('kind'), str):
        raise ValueError('completion must be an explicitly supplied policy')
    needs_reference = completion['kind'] in ['civil_cutoff', 'weekly_civil_cutoff']
    reference_id = completion.get('reference_id')
    if needs_reference or reference_id is not None or completion_reference is not None:
        if (not reference_id or not isinstance(completion_reference, dict)
                or completion_reference.get('id') != reference_id):
            raise ValueError('completion reference must match the supplied source')
    return copy.deepcopy(completion), copy.deepcopy(completion_reference)


def resource_profile(case_id, kind, calendar, completion, *, completion_reference=None):
    """Return one case profile without publishing or deciding its applicability."""
    if kind not in RESOURCE_KINDS:
        raise ValueError('unknown resource corpus kind')
    policy, policy_reference = _completion(completion, completion_reference)
    if kind == 'oral' and policy['kind'] != 'arithmetic_instant':
        raise ValueError('oral hearing end requires arithmetic_instant completion')
    if kind != 'oral' and not isinstance(calendar, dict):
        raise ValueError('daily corpus examples require the supplied calendar snapshot')
    title, quantity, expected_date, locator, specific = _RESOURCES[kind]
    references = _legal_references(kind, locator)
    legal_ids = [row['id'] for row in references]
    if policy_reference is not None:
        references.append(policy_reference)
    statements = [
        'El supuesto corresponde a un organo penal local de CDMX.',
        'La resolucion y el destinatario recurrente estan identificados en la fuente exacta seleccionada.',
        *specific,
        'No concurren notificacion electronica, suspension, reposicion ni abreviacion del plazo.',
    ]
    if kind != 'oral':
        statements.extend([
            'Los efectos declarados corresponden al dia posterior a la notificacion personal; '
            'esa fecha habil se incluye como primer dia del computo.',
            'El calendario seleccionado declara los dias computables y cubre el intervalo sin incidencias pendientes.',
        ])
    if kind == 'oral':
        trigger = {'kind': 'qualified', 'purpose': 'hearing_end', 'family': 'hearing_result'}
        rule = {'kind': 'at_anchor'}
        anchor = {'precision': 'second', 'year': 2026, 'month': 10, 'day': 9,
                  'hour': 10, 'minute': 50, 'second': 0, 'offset_seconds': -21600}
        outcome = {'kind': 'instant_candidate', 'instant': {
            'unix_seconds': 1791564600, 'nanosecond': 0, 'offset_seconds': -21600}}
        example_calendar = None
        example_locator = 'docs/legal-corpus-resources.md: revocacion oral, fin declarado de audiencia'
    else:
        trigger = {'kind': 'source_field', 'field': 'notification_stated_effect_at'}
        rule = {'kind': 'days', 'quantity': quantity, 'inclusion': 'on_anchor',
                'basis': 'calendar_countable', 'final_day': 'preserve'}
        anchor = {'precision': 'date', 'year': 2026, 'month': 10, 'day': 15, 'offset_seconds': None}
        outcome = {'kind': 'civil_candidate', 'date': expected_date}
        example_calendar = copy.deepcopy(calendar)
        example_locator = 'docs/legal-corpus-resources.md: ' + kind + ', enumeracion manual de dias incluidos'
    return {
        'title': title,
        'description': 'Ficha controlada de CDMX en docs/legal-corpus-resources.md. '
                       'Conserva fuente, supuesto y resultado independiente; la disponibilidad '
                       'del perfil no determina procedencia ni admisibilidad del recurso.',
        'scope': {'kind': 'case', 'case_id': case_id}, 'references': references,
        'trigger': trigger, 'template': {'kind': 'fixed', 'rule': rule}, 'completion': policy,
        'conditions': [{'id': str(uuid4()), 'statement': statement,
                        'reference_ids': list(legal_ids)} for statement in statements],
        'examples': [{'id': str(uuid4()), 'anchor': anchor, 'ordered_quantity': None,
                      'calendar': example_calendar, 'expected': {'kind': 'arithmetic', 'outcome': outcome},
                      'reference_ids': list(legal_ids), 'locator': example_locator}],
    }
