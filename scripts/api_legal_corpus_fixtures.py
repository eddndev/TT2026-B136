"""Disposable HTTP inputs for the eight source-qualified legal corpus cases."""
import copy
from uuid import uuid4

from api_deadlines_support import register
from api_hearing_derived_deadline_fixtures import facts, persist, request, route
from api_hearing_results_demo import command as result_command
from legal_corpus_calendar import calendar_values, desk_completion, reference
from legal_corpus_investigation import investigation_profile
from legal_corpus_resources import RESOURCE_KINDS, resource_profile

KINDS = ('initial', 'extension', *RESOURCE_KINDS)
MONTHLY = ('initial', 'extension')
SOURCE_DESCRIPTIONS = {
    'written': 'Acuerdo de mero tramite sin sustanciacion, dictado fuera de audiencia.',
    'control_order': 'Auto del juez de control que niega el anticipo de prueba.',
    'control_sentence': 'Sentencia definitiva del juez de control en procedimiento abreviado.',
    'withdrawal': 'Resolucion del tribunal de enjuiciamiento sobre desistimiento de la '
                  'accion penal del Ministerio Publico; no es retiro de un recurso.',
    'trial_sentence': 'Sentencia definitiva del tribunal de enjuiciamiento, leida y '
                      'explicada en audiencia con version escrita disponible al destinatario.',
}


def _date(month, day):
    return {'precision': 'date', 'year': 2026, 'month': month, 'day': day,
            'offset_seconds': None}


def _known(value):
    return {'kind': 'known', 'value': value}


def _monthly_completion():
    source = reference('Orden judicial ficticia: cierre mensual expreso',
        'Juzgado de control ficticio del corpus; no es autoridad emisora real',
        'https://github.com/eddndev/TT2026-B136/blob/main/docs/legal-corpus-investigation.md',
        'Caso controlado 2026: orden ficticia de cierre 18:00 -06:00; no atribuido al CNPP')
    return ({'kind': 'civil_cutoff', 'time': '18:00:00', 'offset_seconds': -21600,
             'from': '2026-01-01', 'through': '2026-12-31',
             'channel': 'Cierre expresamente ordenado en resolucion ficticia del corpus',
             'reference_id': source['id']}, source)


def fixture():
    """Publish a private corpus using the current Owner, without enrolling accounts."""
    actor = request('GET', '/api/v1/auth/me')
    actor = {key: actor[key] for key in ('id', 'email', 'role')}
    if actor['role'] != 'owner':
        raise ValueError('the disposable corpus requires its current Owner token')
    marker = uuid4().hex[:12]
    case = request('POST', '/api/v1/penal-cases', {
        'title': 'Corpus juridico ficticio ' + marker, 'reference': 'CORPUS-' + marker,
        'profile': {'nuc': 'CORPUS-NUC-' + marker,
                    'nuc_authority': 'Fiscalia ficticia CDMX',
                    'judicial_case_number': 'CORPUS-CJ-' + marker,
                    'judicial_authority': 'Organo penal ficticio CDMX',
                    'offenses': ['Delito ficticio con pena maxima no mayor de dos anios'],
                    'general_information': 'Datos sinteticos para aceptacion aislada',
                    'complementary_identifiers': None}}, expected=201)
    case_id = case['id']
    request('PUT', route(case_id) + '/members/' + actor['id'], expected=204)
    monthly, order = _monthly_completion()
    desk, desk_sources = desk_completion()
    values = calendar_values()
    values['sources'].extend([order, *desk_sources])
    calendar = persist('/api/v1/judicial-calendars', {
        'operation_id': str(uuid4()), 'calendar_id': str(uuid4()), 'change': {
            'action': 'publish', 'expected_revision': 0, 'values': values}})
    profiles = {}
    for kind in KINDS:
        if kind in MONTHLY:
            definition = investigation_profile(case_id, kind, values, monthly)
        elif kind == 'oral':
            definition = resource_profile(case_id, kind, values, {'kind': 'arithmetic_instant'})
        else:
            definition = resource_profile(case_id, kind, values, desk,
                                          completion_reference=desk_sources[0])
            definition['references'].append(copy.deepcopy(desk_sources[1]))
        profiles[kind] = persist(route(case_id) + '/deadline-profiles', {
            'profile_id': str(uuid4()), 'operation_id': str(uuid4()), 'change': {
                'action': 'publish', 'expected_revision': 0, 'definition': definition}})
    return {'actor': actor, 'case': case, 'case_id': case_id,
            'calendar': calendar, 'profiles': profiles}


def _validate_variant(kind, variant):
    if kind not in KINDS or kind == 'oral':
        raise ValueError('source_deadline requires a resolution or notification profile')
    allowed = {'positive', 'scope_rejected', 'condition_rejected', 'missing_anchor'}
    if kind in MONTHLY:
        allowed.update(('missing_duration', 'monthly_no_homologue'))
    else:
        allowed.add('missing_effect')
    if kind == 'extension':
        allowed.update(('monthly_grant_missing', 'excess_accumulated'))
    if kind == 'written':
        allowed.add('written_holiday')
    if variant not in allowed:
        raise ValueError('unsupported corpus variant for ' + kind + ': ' + variant)


def _monthly_description(kind, variant, start):
    duration = 'Duracion concedida expresamente: un mes civil.'
    if variant == 'missing_duration':
        duration = 'La fuente omite la duracion efectivamente concedida; solo consta el maximo.'
    text = ('Orden judicial ficticia de investigacion complementaria. ' + duration
            + ' Inicio ordenado ' + str(start['day']) + '/' + str(start['month'])
            + '/2026. Computo de fecha a fecha, mismo numero de dia, sin sumar ni restar '
            'un dia ni suplir homologos ausentes. Pena maxima no mayor de dos anios; '
            'categoria maxima de dos meses. Cierre expreso 18:00 -06:00; '
            'calendario penal local CDMX identificado. No hay suspension ni cierre anticipado.')
    if kind == 'extension':
        original = '31/12/2025' if variant == 'monthly_no_homologue' else '12/01/2026'
        text += (' Periodo original expresamente identificado: inicio ' + original
                 + ', un mes concedido; sin prorrogas previas. La ampliacion usa '
                 'el inicio ordenado indicado en esta misma fuente.')
        if variant != 'missing_duration':
            text += ' Acumulado expresamente calificado de dos meses desde el inicio original.'
    if variant == 'monthly_grant_missing':
        text = ('Constancia ficticia de mera solicitud de prorroga por un mes desde '
                '12/02/2026; no existe concesion judicial. Periodo original: '
                'un mes desde 12/01/2026, maximo aplicable dos meses.')
    elif variant == 'excess_accumulated':
        text = ('Orden ficticia: periodo original de dos meses desde 12/01/2026; '
                'otra ampliacion de un mes desde 12/03/2026. Acumulado expresamente '
                'calificado de tres meses que excede el maximo aplicable de dos; '
                'misma unidad civil y metodo de fecha a fecha, cierre 18:00 -06:00.')
    elif variant == 'missing_anchor':
        text = ('Orden ficticia de un mes civil; inicio ordenado desconocido. '
                'Maximo aplicable dos meses; metodo de fecha a fecha, cierre 18:00 -06:00.')
    return text


def _resolution(scenario, kind, variant, start):
    value = facts.values('resolution')
    tribunal = kind in ('withdrawal', 'trial_sentence')
    value.update(subtype='Supuesto ficticio ' + kind,
        issuer=_known('Tribunal de enjuiciamiento ficticio CDMX' if tribunal
                      else 'Juez de control ficticio CDMX'), issued_at=copy.deepcopy(start),
        summary=(_monthly_description(kind, variant, start) if kind in MONTHLY
                 else SOURCE_DESCRIPTIONS[kind] + ' Destinatario recurrente ficticio A.'),
        provenance={'kind': 'operator_note',
                    'note': 'Fuente ficticia del corpus: resolucion ' + kind + ', apartado 1.'})
    value['class'] = _known({'kind': 'judgment' if kind in (
        'control_sentence', 'trial_sentence') else 'order'})
    if variant == 'scope_rejected':
        value['issuer'] = _known('Organo ficticio ajeno al ambito penal local CDMX')
    if variant == 'condition_rejected':
        value['summary'] += ' La primera premisa del perfil esta expresamente rechazada.'
    command = facts.record('resolution', value)
    return facts.submit(route(scenario['case_id']), facts.prepare(route(scenario['case_id']), command))


def _notification(scenario, kind, variant, resolution):
    value = facts.values('notification')
    notification_day = 28 if variant == 'written_holiday' else 14
    recipient = _known({'kind': 'unlinked', 'label': 'Destinatario recurrente ficticio A',
                       'description': 'Persona sintetica presente; no corresponde a una identidad real'})
    value.update(resolution={key: resolution[key] for key in ('id', 'revision')},
        context=_known({'kind': 'outside_hearing' if kind == 'written' else 'in_hearing'}),
        practiced_at=_date(10, notification_day), received_at=None,
        intended_recipient=recipient, actual_receiver=copy.deepcopy(recipient),
        representation={'kind': 'not_recorded', 'reason': 'Recibe personalmente el destinatario ficticio'},
        summary=('Constancia ficticia de notificacion personal en instalaciones del organo, '
                 'fuera de audiencia.' if kind == 'written' else
                 'Constancia ficticia de notificacion personal en audiencia, destinatario presente.'),
        provenance={'kind': 'operator_note', 'note': 'Constancia ficticia ' + kind + ', apartado 2.'},
        stated_effect={'at': _date(10, notification_day + 1),
                       'statement': 'Efectos declarados al dia siguiente de la notificacion personal.',
                       'locator': 'Constancia ficticia ' + kind + ', apartado 3: efectos'})
    if kind == 'trial_sentence':
        value['summary'] += ' Audiencia de lectura y explicacion; version escrita disponible en ella.'
    if variant == 'missing_effect':
        value['stated_effect'] = None
    elif variant == 'missing_anchor':
        value['stated_effect']['at'] = {'precision': 'unknown'}
    command = facts.record('notification', value, resolution['id'])
    return facts.submit(route(scenario['case_id']), facts.prepare(route(scenario['case_id']), command))


def _deadline(scenario, kind, source, variant='positive'):
    command = register(scenario['case_id'], scenario['profiles'][kind], source, scenario['actor']['id'])
    change = command['change']
    change['tracking'] = {'profile': 'fixed', 'source': 'fixed',
                          'calendar': 'undetermined' if kind == 'oral' else 'fixed'}
    definition = change['definition']
    definition['title'] = 'Corpus ficticio: ' + kind + ' / ' + variant
    inputs = definition['input']
    if kind != 'oral':
        inputs['calendar'] = {key: scenario['calendar'][key] for key in ('id', 'revision')}
    qualification = inputs['qualification']
    qualification.update(statement='Aplicabilidad expresamente declarada para el supuesto ficticio '
        + kind + '; variante ' + variant + '. No hay incidencias pendientes salvo rechazo indicado.',
        locator='Fuente ficticia ' + kind + ', apartados 1-3 y calificacion de premisas')
    for index, condition in enumerate(qualification['conditions'], 1):
        condition['locator'] = 'Fuente ficticia ' + kind + ': calificacion expresa de premisa ' + str(index)
    if variant == 'scope_rejected':
        qualification['scope_applies']['value'] = False
    elif variant == 'condition_rejected':
        qualification['conditions'][0]['applies']['value'] = False
    elif variant in ('monthly_grant_missing', 'excess_accumulated'):
        index = -2 if variant == 'monthly_grant_missing' else -1
        qualification['conditions'][index]['applies']['value'] = False
    return command


def source_deadline(scenario, kind, variant='positive'):
    """Persist exact procedural sources and return an unsubmitted register command."""
    _validate_variant(kind, variant)
    start = _date(2 if kind == 'extension' else 1, 12) if kind in MONTHLY else _date(10, 14)
    if variant == 'monthly_no_homologue':
        start = _date(1, 31)
    elif variant == 'excess_accumulated':
        start = _date(3, 12)
    elif variant == 'written_holiday':
        start = _date(10, 28)
    resolution = _resolution(scenario, kind, variant, start)
    source = resolution if kind in MONTHLY else _notification(scenario, kind, variant, resolution)
    command = _deadline(scenario, kind, source, variant)
    inputs = command['change']['definition']['input']
    if kind in MONTHLY:
        inputs['ordered_quantity'] = None if variant == 'missing_duration' else 1
        inputs['selection']['qualification'] = {
            'purpose': 'ordered_period_start',
            'at': {'precision': 'unknown'} if variant == 'missing_anchor' else start,
            'statement': 'Inicio expresamente calificado de la orden ficticia seleccionada.',
            'locator': 'Resolucion ficticia ' + kind + ', apartado 1: inicio ordenado'}
    else:
        inputs['selection']['source']['value'] = {
            'family': 'notification', 'id': source['id'], 'revision': source['revision'],
            'resolution': {key: resolution[key] for key in ('id', 'revision')}}
    return command


def compound(scenario):
    """Return the compound result/deadline command anchored at the declared hearing end."""
    collection = route(scenario['case_id'])
    context = request('GET', collection + '/hearings/context')
    hearing = persist(collection + '/hearings', {
        'operation_id': str(uuid4()), 'hearing_id': str(uuid4()), 'change': {
            'action': 'schedule', 'expected_revision': 0,
            'expected_case_revision': context['case_revision'],
            'expected_stage_revision': context['stage_revision'], 'values': {
                'kind': 'initial', 'scheduled_at': '2026-10-09T10:05:00-06:00',
                'modality': 'in_person', 'venue': 'Sala ficticia penal CDMX',
                'note': None, 'participants': [], 'conviction_basis': None}}})
    result = result_command(hearing['id'], hearing['revision'], {
        'occurrence': 'occurred', 'extent': 'concluded',
        'event_time': {'precision': 'instant', 'at': '2026-10-09T10:05:00-06:00'},
        'summary': 'Audiencia ficticia del 09/10/2026. Resolucion de mero tramite sin '
                   'sustanciacion dictada durante la audiencia; destinatario recurrente ficticio '
                   'A presente y personalmente notificado. Fin declarado 10:50:00 -06:00; '
                   'la oportunidad oral es anterior a ese fin. No hay incidencias pendientes.',
        'attendees': [], 'agreements': [],
        'provenance': {'kind': 'operator_note', 'reference': None, 'support': None}})
    deadline = _deadline(scenario, 'oral', {'id': result['result_id'], 'revision': 1})
    selection = deadline['change']['definition']['input']['selection']
    selection['source']['value'] = {'family': 'hearing_result', 'hearing_id': hearing['id'],
        'result_id': result['result_id'], 'revision': 1, 'agreement_id': None}
    selection['qualification'] = {'purpose': 'hearing_end',
        'at': {**_date(10, 9), 'precision': 'second', 'hour': 10, 'minute': 50,
               'second': 0, 'offset_seconds': -21600},
        'statement': 'Fin expreso de la audiencia ficticia; no se infiere de la hora del evento.',
        'locator': 'Resultado ficticio, declaracion de fin de audiencia 09/10/2026'}
    return {'case_id': scenario['case_id'], 'result': result, 'deadline': deadline}
