"""Independent HRES1 vectors; execute to reproduce hearing_result_vectors.json."""
from copy import deepcopy
from datetime import date, datetime, timezone
from hashlib import sha256
from pathlib import Path
from uuid import UUID
import json
import struct


def text(value):
    raw = value.encode("utf-8")
    return struct.pack(">I", len(raw)) + raw


def optional(value, encode):
    return b"\0" if value is None else b"\1" + encode(value)


def declared_time(value):
    if value["precision"] == "date":
        day = date.fromisoformat(value["date"])
        offset = value["offset"]
        seconds = (int(offset[1:3]) * 60 + int(offset[4:6])) * 60
        seconds *= -1 if offset.startswith("-") else 1
        return b"\0" + struct.pack(">HBBi", day.year, day.month, day.day, seconds)
    stamp = datetime.fromisoformat(value["at"])
    epoch = datetime(1970, 1, 1, tzinfo=timezone.utc)
    delta = stamp.astimezone(timezone.utc) - epoch
    return b"\1" + struct.pack(">qi", delta.days * 86400 + delta.seconds,
                                int(stamp.utcoffset().total_seconds()))


def support(value):
    return UUID(value["document_id"]).bytes + struct.pack(">I", value["version"]) + bytes.fromhex(value["digest"])


def encode(value):
    body = b"HRES1" + bytes([['occurred', 'not_started'].index(value['occurrence']),
                              ['partial', 'concluded', 'unspecified'].index(value['extent'])])
    body += declared_time(value['event_time']) + text(value['summary'])
    attendees = sorted(value['attendees'], key=lambda item: UUID(item['participant_id']).bytes)
    body += bytes([len(attendees)])
    for item in attendees:
        body += UUID(item['participant_id']).bytes + struct.pack('>I', item['revision'])
        body += text(item['capacity']) + optional(item['observation'], text)
    body += bytes([len(value['agreements'])])
    for item in value['agreements']:
        body += UUID(item['id']).bytes + text(item['text'])
    source = value['provenance']
    body += bytes([['operator_note', 'oral_reference', 'written_record'].index(source['kind'])])
    body += optional(source['reference'], text) + optional(source['support'], support)
    return body


def row(name, value):
    wire = encode(value)
    return {'name': name, 'input': value, 'bytes': len(wire), 'hex': wire.hex(),
            'sha256': sha256(wire).hexdigest()}


base = {'occurrence': 'occurred', 'extent': 'unspecified',
        'event_time': {'precision': 'date', 'date': '0001-01-01', 'offset': '+00:00'},
        'summary': 'x', 'attendees': [], 'agreements': [],
        'provenance': {'kind': 'operator_note', 'reference': None, 'support': None}}
assert len(encode(base)) == 26
rows = [row('minimum_date', base)]
partial = deepcopy(base)
partial.update(extent='partial', summary='Declared\nResult \u00e1',
               event_time={'precision': 'instant', 'at': '1900-01-01T12:34:56+05:30'})
partial['attendees'] = [dict(participant_id=str(UUID(int=9)), revision=7, capacity='Witness', observation='Arrived late'),
                       dict(participant_id=str(UUID(int=1)), revision=0xffffffff, capacity='Counsel', observation=None)]
partial['agreements'] = [dict(id=str(UUID(int=2)), text='Second identifier first'),
                         dict(id=str(UUID(int=1)), text='First identifier second')]
partial['provenance'] = dict(kind='oral_reference', reference='Recording 00:12:00', support=dict(
    document_id='00112233-4455-6677-8899-aabbccddeeff', version=7, digest=bytes(range(32)).hex()))
rows.append(row('partial_negative_epoch_oral_support', partial))
not_started = deepcopy(partial)
not_started.update(occurrence='not_started', extent='unspecified',
                   event_time={'precision': 'date', 'date': '2026-01-01', 'offset': '+14:00'})
not_started['provenance'].update(kind='written_record', reference='Minutes, page 2', support=None)
rows.append(row('not_started_with_attendance_and_agreements', not_started))
concluded = deepcopy(partial)
concluded.update(extent='concluded', event_time={'precision': 'instant', 'at': '2026-12-31T23:50:00-14:00'})
rows.append(row('concluded_next_utc_year', concluded))
same_instant = deepcopy(concluded)
same_instant['event_time']['at'] = '2027-01-01T13:50:00+00:00'
rows.append(row('same_instant_distinct_offset', same_instant))
maximum = deepcopy(concluded)
wide = chr(0x1f642)
maximum['summary'] = wide * 1000
maximum['attendees'] = [dict(participant_id=str(UUID(int=n)), revision=0xffffffff,
                              capacity=wide * 100, observation=wide * 500) for n in range(32, 0, -1)]
maximum['agreements'] = [dict(id=str(UUID(int=n)), text=wide * 1000) for n in range(16, 0, -1)]
maximum['provenance'].update(kind='written_record', reference=wide * 200)
maximum['provenance']['support']['version'] = 0xffffffff
assert len(encode(maximum)) == 146933
rows.append(row('maximum_utf8', maximum))
Path(__file__).with_name('hearing_result_vectors.json').write_text(
    '[\n' + ',\n'.join(json.dumps(item, ensure_ascii=True, separators=(",", ":")) for item in rows) + '\n]\n', encoding='ascii')
print(json.dumps({'vectors': len(rows), 'minimum_bytes': len(encode(base)),
                  'maximum_bytes': len(encode(maximum))}))
