"""Check independent SQL decoding in an explicitly disposable PostgreSQL schema."""
from copy import deepcopy
import json
import struct
import unittest
from uuid import UUID
from judicial_calendar_sql_support import CalendarSqlFixture, ORACLE, blob, raw_values


class CalendarSqlTests(CalendarSqlFixture):
    def test_source_url_profile_rejects_ambiguous_and_unsupported_input(self):
        valid = ['https://example.invalid', 'https://EXAMPLE.invalid/path?x=%20#anchor',
                 "https://a.gob.mx/p!$&'()*+,;=:@/?q=~#a?b"]
        invalid = ['', 'http://example.invalid', 'HTTPS://example.invalid',
                   'https://example.invalid:443/', 'https://user@example.invalid/',
                   'https://127.0.0.1/', 'https://[::1]/', 'https://localhost/',
                   'https://xn--bcher-kva.example/', 'https://bad-.example/',
                   'https://bad..example/', 'https://example.invalid/%ZZ',
                   'https://example.invalid/%2', 'https://example.invalid/{a}',
                   'https://example.invalid/a b', 'https://example.invalid/a\\b',
                   'https://example.invalid/#a#b', 'https://example.invalid/"x"',
                   'https://example.invalid/' + '\u00e9']
        for value, expected in [(s, 't') for s in valid] + [(s, 'f') for s in invalid]:
            with self.subTest(value=value):
                literal = value.replace("'", "''")
                self.assertEqual(self.sql("SELECT judicial_calendar_url_valid('" + literal + "')"), expected)

    def test_civil_dates_use_iso_independently_from_session_datestyle(self):
        for style in ['ISO, YMD', 'SQL, DMY', 'Postgres, MDY', 'German, DMY']:
            for number, expected in [(-719162, '0001-01-01'), (0, '1970-01-01'),
                                     (19782, '2024-02-29'), (2932896, '9999-12-31')]:
                raw = struct.pack('>i', number).hex()
                value = self.sql("SET DateStyle='" + style + "'; SELECT judicial_calendar_date(decode('" + raw + "','hex'),0)")
                self.assertEqual(json.loads(value), {'next': 4, 'value': expected})
        for raw in ['000000', struct.pack('>i', -719163).hex(), struct.pack('>i', 2932897).hex()]:
            self.sql("SELECT judicial_calendar_date(decode('" + raw + "','hex'),0)", success=False)

    def test_all_independent_canonical_vectors_decode_exactly(self):
        for vector in self.vectors:
            with self.subTest(vector=vector['name']):
                actual = self.sql('SELECT judicial_calendar_values(' + blob(bytes.fromhex(vector['hex'])) + ')')
                self.assertEqual(json.loads(actual), vector['normalized'])

    def test_values_reject_truncation_tags_and_noncanonical_text(self):
        base = bytes.fromhex(self.vectors[0]['hex'])
        for raw in [b'', base[:-1], base + b'x', b'BAD!!' + base[5:],
                    base[:10] + b'\x02' + base[11:], base[:11] + b'\x00' + base[12:],
                    base[:12] + b'\x00' + base[13:], base[:42] + b'\x00' + base[43:]]:
            self.reject_bytes('judicial_calendar_values', raw)
        for text in ['', ' x', 'x ', 'x\n', 'x\r\ny', 'x\x7fy', 'x\u0085y', 'x' * 201]:
            value = ORACLE.minimum()
            value['scope']['title'] = text
            self.reject_bytes('judicial_calendar_values', raw_values(value))
        raw = base[:5] + struct.pack('>I', 1) + b'\xff' + base[10:]
        self.reject_bytes('judicial_calendar_values', raw)
        self.reject_bytes('judicial_calendar_values', base[:5] + b'\xff' * 4 + base[9:])

    def test_sources_rules_and_exception_cross_fields_are_strict(self):
        base = self.vectors[3]['normalized']
        invalid = []
        for field in ['sources', 'weekly_pattern', 'exceptions']:
            value = deepcopy(base)
            value[field].reverse()
            invalid.append(value)
        for codes in [['01', '01'], ['32', '01'], ['00'], ['33']]:
            value = deepcopy(base)
            value['scope']['entity_codes'] = codes
            invalid.append(value)
        for changed in [dict(weekday=0), dict(weekday=7), dict(source_ids=[]),
                        dict(source_ids=[str(UUID(int=999))]), dict(source_ids=[str(UUID(int=0))] * 2),
                        dict(source_ids=list(reversed(base['weekly_pattern'][0]['source_ids'])))]:
            value = deepcopy(base)
            value['weekly_pattern'][0].update(changed)
            invalid.append(value)
        for changed in [dict(published_on='2001-01-01'), dict(official_url=' https://example.invalid'),
                        dict(official_url='https://example.invalid/%QQ'), dict(locator='x' * 513),
                        dict(id=base['sources'][1]['id'])]:
            value = deepcopy(base)
            value['sources'][0].update(changed)
            invalid.append(value)
        for changed in [{'from': '2000-02-26'}, {'through': '2000-03-05'}, {'through': '2000-02-28'},
                        {'through': '2000-03-02'}, {'id': base['exceptions'][1]['id']}]:
            value = deepcopy(base)
            value['exceptions'][0].update(changed)
            invalid.append(value)
        for coverage in [{'from': '2000-01-02', 'through': '2000-01-01'},
                         {'from': '2000-01-01', 'through': '2003-01-01'}]:
            value = ORACLE.minimum()
            value['coverage'] = coverage
            invalid.append(value)
        for index, value in enumerate(invalid):
            with self.subTest(index=index):
                self.reject_bytes('judicial_calendar_values', raw_values(value))

    def test_receipts_match_independent_oracle_and_reject_incoherent_variants(self):
        receipts = ORACLE.report(ORACLE.vectors())['receipts']
        for receipt in receipts:
            value = receipt['input']
            expected = dict(actor_id=value['actor'], operation_id=value['operation'], calendar_id=value['calendar'],
                            action=value['action'], expected_revision=value['expected'],
                            values_digest=value['digest'], reason=value['reason'])
            actual = self.sql('SELECT judicial_calendar_submission(' + blob(bytes.fromhex(receipt['hex'])) + ')')
            self.assertEqual(json.loads(actual), expected)
        base = bytes.fromhex(receipts[0]['hex'])
        for raw in [b'', base[:-1], base + b'x', b'BAD!!' + base[5:],
                    base[:53] + b'\x03' + base[54:], base[:54] + b'\xff' * 4 + base[58:],
                    base[:53] + b'\x01' + base[54:], base[:54] + b'\0\0\0\1' + base[58:],
                    base[:-1] + b'\x02', base[:-1] + b'\x01' + ORACLE.text('x')]:
            self.reject_bytes('judicial_calendar_submission', raw)
        replace = bytes.fromhex(receipts[1]['hex'])
        for raw in [replace[:90] + b'\0', replace[:54] + b'\xff' * 4 + replace[58:],
                    replace[:91] + ORACLE.text(' x'), replace[:91] + ORACLE.text(''),
                    replace[:91] + ORACLE.text('x' * 1001)]:
            self.reject_bytes('judicial_calendar_submission', raw)

    def test_decoders_reject_count_overflows_and_invalid_options(self):
        base = bytes.fromhex(self.vectors[0]['hex'])
        for offset, number in [(11, 33), (41, 17), (43, 3), (44, 17), (98, 65)]:
            self.reject_bytes('judicial_calendar_values', base[:offset] + bytes([number]) + base[offset+1:])
        # Standalone nested decoders must reject invalid offsets and null input too.
        for function in ['judicial_calendar_source', 'judicial_calendar_rule', 'judicial_calendar_date']:
            for args in ['NULL,0', blob(base) + ',NULL', blob(base) + ',-1', blob(base) + ',2147483647']:
                self.reject('PERFORM ' + function + '(' + args + ')')
        value = deepcopy(self.vectors[3]['normalized'])
        from judicial_calendar_sql_support import source_bytes
        raw = source_bytes(value['sources'][0])
        source = value['sources'][0]
        option_offset = 16 + sum(4 + len(source[k].encode('utf-8')) for k in ['title', 'issuer', 'official_url'])
        for malformed in [raw[:option_offset] + b'\x02' + raw[option_offset+1:], raw[:-1]]:
            self.reject('PERFORM judicial_calendar_source(' + blob(malformed) + ',0)')

    def test_full_values_projection_does_not_depend_on_session_datestyle(self):
        for style in ['SQL, DMY', 'Postgres, MDY', 'German, DMY']:
            for index in [1, 2, 3, 6]:
                vector = self.vectors[index]
                actual = self.sql("SET DateStyle='" + style + "';SELECT judicial_calendar_values(" + blob(bytes.fromhex(vector['hex'])) + ')')
                self.assertEqual(json.loads(actual), vector['normalized'])


if __name__ == '__main__':
    from test_judicial_calendar_concurrency import CalendarConcurrencyTests
    from test_judicial_calendar_schema import CalendarSchemaTests
    unittest.main()
