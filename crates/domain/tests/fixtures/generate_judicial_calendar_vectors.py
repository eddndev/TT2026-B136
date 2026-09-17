"""Independent JCAL1/JCTX1 vectors from docs/judicial-calendars-api.md."""
from argparse import ArgumentParser
from copy import deepcopy
from datetime import date, timedelta
from hashlib import sha256
from pathlib import Path
from uuid import UUID
import json
import re
import struct
import unicodedata


CLASSIFICATIONS = ['countable', 'excluded', 'unresolved']
SCOPE_TEXTS = ['title', 'authority', 'organ', 'territory', 'use_description']
EPOCH = date(1970, 1, 1)


def require(condition, label):
    if not condition:
        raise ValueError(label)


def day(value):
    require(isinstance(value, str) and re.fullmatch(r'[0-9]{4}-[0-9]{2}-[0-9]{2}', value), 'date shape')
    return date.fromisoformat(value)


def clean(value, limit, multiline=False):
    require(isinstance(value, str), 'text type')
    value = value.replace('\r\n', '\n')
    require(all(unicodedata.category(c) not in ('Cc', 'Cs') or (c == '\n' and multiline)
                for c in value), 'text control')
    value = value.strip()
    require(1 <= len(value) <= limit, 'text length')
    return value


def official_url(value):
    value = clean(value, 2048)
    require(value.isascii() and value.startswith('https://'), 'URL scheme')
    host, tail = re.match(r'([^/?#]*)(.*)', value[8:]).groups()
    labels = host.split('.')
    require(2 <= len(labels) and len(host) <= 253, 'URL host')
    require(all(re.fullmatch(r'[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?', label)
                and not label.lower().startswith('xn--') for label in labels), 'URL DNS label')
    require(re.fullmatch(r'[A-Za-z]{2,63}', labels[-1]), 'URL TLD')
    require(tail.count('#') <= 1, 'URL fragment')
    require(re.fullmatch(r"(?:[A-Za-z0-9\-._~!$&'()*+,;=:@/?#]|%[A-Fa-f0-9]{2})*", tail), 'URL tail')
    return value


def identifiers(values):
    result = sorted((str(UUID(v)) for v in values), key=lambda v: UUID(v).bytes)
    require(len(result) == len(set(result)), 'duplicate UUID')
    return result


def normalize(value):
    result = deepcopy(value)
    scope = result['scope']
    for field in SCOPE_TEXTS:
        scope[field] = clean(scope[field], 1000 if field == 'use_description' else 200,
                             field == 'use_description')
    require(scope['jurisdiction'] in ['federal', 'local'], 'jurisdiction')
    codes = scope['entity_codes']
    require(1 <= len(codes) <= 32, 'entity count')
    require(all(isinstance(c, str) and re.fullmatch(r'[0-9]{2}', c) and 1 <= int(c) <= 32 for c in codes), 'entity code')
    require(len(codes) == len(set(codes)), 'duplicate entity')
    scope['entity_codes'] = sorted(codes)
    first, last = (day(result['coverage'][k]) for k in ['from', 'through'])
    require(1 <= (last - first).days + 1 <= 1096, 'coverage')
    require(len(result['sources']) <= 16, 'source count')
    source_ids = identifiers([s['id'] for s in result['sources']])
    for source in result['sources']:
        source['id'] = str(UUID(source['id']))
        for key, limit in [('title', 200), ('issuer', 200), ('locator', 512)]:
            source[key] = clean(source[key], limit)
        source['official_url'] = official_url(source['official_url'])
        consulted = day(source['consulted_on'])
        published = source.setdefault('published_on', None)
        require(published is None or day(published) <= consulted, 'publication date')
    result['sources'].sort(key=lambda s: UUID(s['id']).bytes)
    week = result['weekly_pattern']
    require(len(week) == 7 and all(type(r['weekday']) is int for r in week)
            and sorted(r['weekday'] for r in week) == list(range(1, 8)), 'weekdays')
    week.sort(key=lambda r: r['weekday'])
    exceptions = result['exceptions']
    require(len(exceptions) <= 64, 'exception count')
    identifiers([e['id'] for e in exceptions])
    for exception in exceptions:
        exception['id'] = str(UUID(exception['id']))
        require(first <= day(exception['from']) <= day(exception['through']) <= last, 'exception coverage')
    exceptions.sort(key=lambda e: (day(e['from']), day(e['through']), UUID(e['id']).bytes))
    require(all(day(a['through']) < day(b['from']) for a, b in zip(exceptions, exceptions[1:])), 'exception overlap')
    for rule in week + exceptions:
        require(rule['classification'] in CLASSIFICATIONS, 'classification')
        rule['source_ids'] = identifiers(rule['source_ids'])
        require(len(rule['source_ids']) <= 16 and set(rule['source_ids']) <= set(source_ids), 'rule references')
        require(rule['classification'] == 'unresolved' or rule['source_ids'], 'missing rule source')
        rule['explanation'] = clean(rule['explanation'], 256, True)
    return result


def text(value):
    raw = value.encode('utf-8')
    return struct.pack('>I', len(raw)) + raw


def civil(value):
    return struct.pack('>i', (day(value) - EPOCH).days)


def rule_bytes(rule):
    refs = rule['source_ids']
    return (bytes([CLASSIFICATIONS.index(rule['classification']), len(refs)])
            + b''.join(UUID(v).bytes for v in refs) + text(rule['explanation']))


def encode(value):
    value = normalize(value)
    scope = value['scope']
    body = b'JCAL1' + text(scope['title'])
    body += bytes([['federal', 'local'].index(scope['jurisdiction']), len(scope['entity_codes'])])
    body += bytes(int(c) for c in scope['entity_codes'])
    body += b''.join(text(scope[k]) for k in SCOPE_TEXTS[1:])
    body += civil(value['coverage']['from']) + civil(value['coverage']['through'])
    body += bytes([len(value['sources'])])
    for source in value['sources']:
        body += UUID(source['id']).bytes
        body += b''.join(text(source[k]) for k in ['title', 'issuer', 'official_url'])
        published = source['published_on']
        body += b'\0' if published is None else b'\1' + civil(published)
        body += civil(source['consulted_on']) + text(source['locator'])
    for rule in value['weekly_pattern']:
        body += bytes([rule['weekday']]) + rule_bytes(rule)
    body += bytes([len(value['exceptions'])])
    for exception in value['exceptions']:
        body += UUID(exception['id']).bytes + civil(exception['from']) + civil(exception['through'])
        body += rule_bytes(exception)
    return body


def submission(actor, operation, calendar, action, expected, digest, reason=None):
    require(type(expected) is int and 0 <= expected < 0xffffffff, 'revision')
    require(action in ['publish', 'replace', 'retire'], 'action')
    require((action == 'publish') == (expected == 0), 'expected revision')
    require((action == 'publish') == (reason is None), 'reason presence')
    digest = bytes.fromhex(digest)
    require(len(digest) == 32, 'digest length')
    body = b'JCTX1' + b''.join(UUID(v).bytes for v in [actor, operation, calendar])
    body += bytes([['publish', 'replace', 'retire'].index(action)]) + struct.pack('>I', expected) + digest
    return body + (b'\0' if reason is None else b'\1' + text(clean(reason, 1000, True)))


def minimum():
    return {'scope': dict(title='x', jurisdiction='federal', entity_codes=['01'],
                          authority='x', organ='x', territory='x', use_description='x'),
            'coverage': {'from': '1970-01-01', 'through': '1970-01-01'}, 'sources': [],
            'weekly_pattern': [dict(weekday=n, classification='unresolved', source_ids=[], explanation='x')
                               for n in range(1, 8)], 'exceptions': []}


def maximum():
    value = minimum()
    wide = chr(0x1f642)
    value['scope'].update({k: wide * (1000 if k == 'use_description' else 200) for k in SCOPE_TEXTS})
    value['scope']['entity_codes'] = [f'{n:02}' for n in range(32, 0, -1)]
    value['coverage'] = {'from': '2000-01-01', 'through': '2002-12-31'}
    refs = [str(UUID(int=n)) for n in range(16, 0, -1)]
    prefix = 'https://example.invalid/'
    value['sources'] = [dict(id=ref, title=wide * 200, issuer=wide * 200,
                             official_url=prefix + 'a' * (2048 - len(prefix)),
                             published_on='0001-01-01', consulted_on='9999-12-31',
                             locator=wide * 512) for ref in refs]
    for item in value['weekly_pattern']:
        item.update(source_ids=refs[:], explanation=wide * 256)
    value['weekly_pattern'].reverse()
    value['exceptions'] = [dict(id=str(UUID(int=n)),
        **{'from': (date(2000, 1, 1) + timedelta(days=n - 1)).isoformat(),
           'through': (date(2000, 1, 1) + timedelta(days=n - 1)).isoformat()},
        classification='unresolved', source_ids=refs[:], explanation=wide * 256) for n in range(64, 0, -1)]
    return value


def classify(value, stamp):
    value = normalize(value)
    d = day(stamp)
    if not day(value['coverage']['from']) <= d <= day(value['coverage']['through']):
        return {'date': stamp, 'state': 'outside_coverage', 'origin': None, 'source_ids': []}
    found = next((e for e in value['exceptions'] if day(e['from']) <= d <= day(e['through'])), None)
    rule = found if found else value['weekly_pattern'][d.isoweekday() - 1]
    result = dict(date=stamp, state=rule['classification'], origin='exception' if found else 'weekly_pattern',
                  explanation=rule['explanation'], source_ids=rule['source_ids'])
    result['exception_id' if found else 'weekday'] = found['id'] if found else d.isoweekday()
    return result


def row(name, value, samples=()):
    raw = encode(value)
    return dict(name=name, input=value, normalized=normalize(value), bytes=len(raw), hex=raw.hex(),
                sha256=sha256(raw).hexdigest(), classifications=[classify(value, d) for d in samples])


def vectors():
    base = minimum()
    rows = [row('minimum_epoch', base, ['1969-12-31', '1970-01-01', '1970-01-02'])]
    for name, stamp in [('minimum_year', '0001-01-01'), ('maximum_year', '9999-12-31')]:
        value = deepcopy(base)
        value['coverage'] = {'from': stamp, 'through': stamp}
        rows.append(row(name, value, [stamp]))
    value = deepcopy(base)
    value['scope'].update(title=' Calendario \u00e1 ', jurisdiction='local', entity_codes=['32', '01'],
                          use_description='\r\nTexto e\u0301 \U0001f642\r\nfin\r\n')
    value['coverage'] = {'from': '2000-02-27', 'through': '2000-03-04'}
    refs = ['00112233-4455-6677-8899-aabbccddeeff', str(UUID(int=0))]
    value['sources'] = [dict(id=ref, title='Norma \u00e9', issuer='Emisor',
                             official_url=' https://example.invalid/a?edition=1#p2 ',
                             published_on='1900-03-01' if n == 0 else None,
                             consulted_on='2000-02-29', locator='Pagina 1') for n, ref in enumerate(refs)]
    for rule in value['weekly_pattern']:
        rule.update(classification='countable' if rule['weekday'] < 6 else 'excluded',
                    source_ids=refs[:], explanation='Regla\r\ndeclarada')
    value['weekly_pattern'].reverse()
    value['exceptions'] = [dict(id=str(UUID(int=2)), **{'from': '2000-03-02', 'through': '2000-03-03'},
        classification='unresolved', source_ids=[], explanation='Pendiente'),
        dict(id=str(UUID(int=0)), **{'from': '2000-02-29', 'through': '2000-02-29'},
        classification='excluded', source_ids=[refs[1]], explanation='Excepcion declarada')]
    samples = ['2000-02-26', '2000-02-27', '2000-02-28', '2000-02-29', '2000-03-02', '2000-03-05']
    rows.append(row('leap_unicode_unordered', value, samples))
    ordered = normalize(value)
    rows.append(row('same_values_canonical_order', ordered, samples))
    distinct = deepcopy(ordered)
    distinct['scope']['use_description'] = distinct['scope']['use_description'].replace('e\u0301', '\u00e9')
    rows.append(row('unicode_composition_is_preserved', distinct))
    rows.append(row('maximum_utf8', maximum(), ['1999-12-31', '2000-02-29', '2002-12-31', '2003-01-01']))
    return rows


def report(rows):
    big = rows[-1]
    ids = [str(UUID(int=n)) for n in [0, 0x123456789abcdef, 0xffffffffffffffffffffffffffffffff]]
    receipts = []
    for action, expected, reason in [('publish', 0, None), ('replace', 0xfffffffe, chr(0x1f642) * 1000), ('retire', 1, 'Retiro')]:
        args = dict(actor=ids[0], operation=ids[1], calendar=ids[2], action=action,
                    expected=expected, digest=big['sha256'], reason=reason)
        raw = submission(**args)
        receipts.append(dict(input=args, bytes=len(raw), hex=raw.hex(), sha256=sha256(raw).hexdigest()))
    command = dict(operation_id=ids[1], calendar_id=ids[2], change=dict(action='replace',
                   expected_revision=0xfffffffe, values=big['normalized'], reason=chr(0x1f642) * 1000))
    envelope = dict(command=command, expected_submission_digest=receipts[1]['sha256'])
    sizes = {name: len(json.dumps(envelope, ensure_ascii=escaped, separators=(',', ':')).encode())
             for name, escaped in [('literal_utf8', False), ('ascii_escaped', True)]}
    require(max(sizes.values()) < 1048576, 'HTTP envelope exceeds limit')
    return dict(values_vectors=len(rows), minimum_bytes=min(r['bytes'] for r in rows),
                maximum_bytes=big['bytes'], source_maximum_bytes=5737, weekly_maximum_bytes=1287,
                exception_maximum_bytes=1310, receipts=receipts, json_command_bytes=sizes,
                http_body_limit=1048576, url_profile='literal HTTPS, restricted ASCII DNS and RFC3986 tail; no network access')


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    rows = vectors()
    require(rows[0]['bytes'] == 99 and rows[-1]['bytes'] == 191910, 'canonical size')
    require(rows[3]['hex'] == rows[4]['hex'] and rows[4]['hex'] != rows[5]['hex'], 'normalization')
    output = '[\n' + ',\n'.join(json.dumps(r, ensure_ascii=True, separators=(',', ':')) for r in rows) + '\n]\n'
    target = Path(__file__).with_name('judicial_calendar_vectors.json')
    if args.check:
        require(target.read_text(encoding='ascii') == output, 'fixture differs')
    else:
        target.write_text(output, encoding='ascii')
    measured = report(rows)
    if args.report:
        args.report.write_text(json.dumps(measured, indent=2, ensure_ascii=True) + '\n', encoding='ascii')
    print(json.dumps({k: v for k, v in measured.items() if k != 'receipts'}))


if __name__ == '__main__':
    main()
