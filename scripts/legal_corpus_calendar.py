"""Source-grounded CDMX ordinary filing coverage for the controlled 2026 corpus."""
from uuid import uuid4

CNPP = 'https://www.diputados.gob.mx/LeyesBiblio/pdf/CNPP.pdf'
CALENDAR = 'https://www.poderjudicialcdmx.gob.mx/wp-content/uploads/Circula_CJCDMX_21_2025.pdf'
DESK = ('https://www.poderjudicialcdmx.gob.mx/wp-content/uploads/'
        'CIRCULAR_CJCDMX_23_2025_Acuerdo_80_29_2025.pdf')
ZONE = 'https://www.diputados.gob.mx/LeyesBiblio/pdf/LHHEUM.pdf'


def reference(title, issuer, url, locator, published=None):
    return {'id': str(uuid4()), 'title': title, 'issuer': issuer, 'official_url': url,
            'published_on': published, 'consulted_on': '2026-10-11', 'locator': locator}


def calendar_values():
    law = reference('CNPP: computo ordinario', 'Congreso de la Union', CNPP,
                    'Articulo 94; reforma 28-11-2025')
    holidays = reference('Inhabiles CDMX 2026', 'Consejo de la Judicatura CDMX', CALENDAR,
                         'Circular 21/2025, pp. 1-4; guardias penales y vacaciones escalonadas',
                         '2025-08-27')
    # Penal vacations are staggered; do not import the civil suspension as a holiday range.
    excluded = ['2026-01-01', '2026-02-02', '2026-03-16', '2026-03-30', '2026-03-31',
                '2026-04-01', '2026-04-02', '2026-04-03', '2026-05-01', '2026-05-04',
                '2026-05-08', '2026-06-19', '2026-09-14', '2026-09-15', '2026-09-16',
                '2026-10-30', '2026-11-02', '2026-11-16']
    return {
        'scope': {'title': 'CDMX penal: corpus ordinario 2026', 'jurisdiction': 'local',
                  'authority': 'Tribunal Superior de Justicia de la Ciudad de Mexico',
                  'organ': 'Unidades de Gestion Judicial penal', 'territory': 'Ciudad de Mexico',
                  'entity_codes': ['09'], 'use_description':
                  'Supuestos ordinarios CNPP sin excepcion urgente ni suspension particular'},
        'coverage': {'from': '2026-01-01', 'through': '2026-11-30'},
        'sources': [law, holidays],
        'weekly_pattern': [{'weekday': day, 'classification': 'countable' if day <= 5 else 'excluded',
                            'source_ids': [law['id']], 'explanation':
                            'CNPP 94: excluye sabados y domingos del supuesto ordinario'}
                           for day in range(1, 8)],
        'exceptions': [{'id': str(uuid4()), 'from': day, 'through': day, 'classification': 'excluded',
                        'source_ids': [holidays['id']],
                        'explanation': 'Inhabil publicado; no aplica una excepcion urgente del CNPP 94'}
                       for day in excluded],
    }


def desk_completion():
    source = reference('Recepcion ordinaria de promociones UGJ', 'Consejo de la Judicatura CDMX',
                       DESK, 'Acuerdo 80-29/2025, articulo 4.I, pagina 13', '2025-08-27')
    zone = reference('Zona Centro', 'Congreso de la Union', ZONE,
                     'Articulos 3.I y 4; horario estacional fronterizo no aplicable a CDMX')
    return ({'kind': 'weekly_civil_cutoff',
             'times': ['15:00:00'] * 4 + ['14:00:00', None, None],
             'offset_seconds': -21600, 'from': '2026-01-01', 'through': '2026-11-30',
             'channel': 'Ventanilla ordinaria UGJ CDMX; cierre del canal, no caducidad universal',
             'reference_id': source['id']}, [source, zone])
