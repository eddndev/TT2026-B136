"""Disposable SQL fixtures and independent encodings for calendar migrations."""
from copy import deepcopy
from hashlib import sha256
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import unittest
from uuid import UUID, uuid4

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('calendar_vectors', ROOT / 'crates/domain/tests/fixtures/generate_judicial_calendar_vectors.py')
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)
MIGRATIONS = ['primitives', 'sources', 'values', 'receipts', 'tables', 'guards']
HELPERS = ['judicial_calendar_url_valid(text)', 'judicial_calendar_date(bytea,integer)',
           'judicial_calendar_source(bytea,integer)', 'judicial_calendar_rule(bytea,integer)',
           'judicial_calendar_values(bytea)', 'judicial_calendar_submission(bytea)']


def literal(value):
    return "'" + value.replace("'", "''") + "'"


def blob(value):
    return "decode('" + value.hex() + "','hex')"


def source_bytes(source):
    body = UUID(source['id']).bytes
    body += b''.join(ORACLE.text(source[k]) for k in ['title', 'issuer', 'official_url'])
    body += b'\0' if source['published_on'] is None else b'\1' + ORACLE.civil(source['published_on'])
    return body + ORACLE.civil(source['consulted_on']) + ORACLE.text(source['locator'])


def raw_values(value):
    """Encode without normalizing so invalid structural inputs reach the SQL decoder."""
    scope = value['scope']
    body = b'JCAL1' + ORACLE.text(scope['title'])
    body += bytes([['federal', 'local'].index(scope['jurisdiction']), len(scope['entity_codes'])])
    body += bytes(int(c) for c in scope['entity_codes'])
    body += b''.join(ORACLE.text(scope[k]) for k in ORACLE.SCOPE_TEXTS[1:])
    body += ORACLE.civil(value['coverage']['from']) + ORACLE.civil(value['coverage']['through'])
    body += bytes([len(value['sources'])]) + b''.join(source_bytes(s) for s in value['sources'])
    for rule in value['weekly_pattern']:
        body += bytes([rule['weekday']]) + ORACLE.rule_bytes(rule)
    body += bytes([len(value['exceptions'])])
    for item in value['exceptions']:
        body += UUID(item['id']).bytes + ORACLE.civil(item['from']) + ORACLE.civil(item['through'])
        body += ORACLE.rule_bytes(item)
    return body


class CalendarSqlFixture(unittest.TestCase):
    @classmethod
    def sql(cls, statement, success=True):
        completed = subprocess.run(['psql', cls.url, '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'],
            input='SET search_path TO ' + cls.schema + ',pg_catalog;\n' + statement,
            text=True, capture_output=True)
        if success and completed.returncode:
            raise AssertionError(completed.stderr)
        if not success and not completed.returncode:
            raise AssertionError('Invalid SQL input was accepted')
        return completed.stdout.strip()

    @classmethod
    def setUpClass(cls):
        cls.url = os.environ.get('DOCUMENT_TEST_DATABASE_URL')
        if not cls.url:
            raise RuntimeError('An isolated DOCUMENT_TEST_DATABASE_URL is required')
        cls.schema = 'calendar_sql_' + uuid4().hex
        cls.sql('CREATE SCHEMA ' + cls.schema)
        installer = (ROOT / 'crates/infrastructure/src/postgres.rs').read_text()
        prerequisites = re.findall(r'include_str!\("../../../(migrations/[^"\n]+)"\)', installer)
        cls.sql('\n'.join((ROOT / p).read_text() for p in prerequisites if not Path(p).name.startswith('0013_')))
        for suffix in MIGRATIONS:
            path = ROOT / 'migrations' / ('0013_judicial_calendar_' + suffix + '.sql')
            if path.exists():
                cls.sql(path.read_text())
        cls.vectors = json.loads((ROOT / 'crates/domain/tests/fixtures/judicial_calendar_vectors.json').read_text())

    @classmethod
    def tearDownClass(cls):
        cls.sql('DROP SCHEMA ' + cls.schema + ' CASCADE')

    @classmethod
    def reject(cls, statement, condition='check_violation'):
        cls.sql('DO $test$ BEGIN ' + statement + '; RAISE EXCEPTION \'unexpected acceptance\'; '
                'EXCEPTION WHEN ' + condition + ' THEN NULL; END; $test$;')

    def reject_bytes(self, function, raw):
        self.reject('PERFORM ' + function + '(' + blob(raw) + ')')

    def actor(self, role='owner', active=True):
        actor = str(uuid4())
        email = actor + '@example.invalid'
        self.sql("INSERT INTO users(id,email,password_hash,role,active,protected_totp_secret,recovery_codes) VALUES ("
                 + ','.join([literal(actor), literal(email), "'test'", literal(role), str(active), "''", "'[]'"]) + ')')
        return actor, email

    def insert(self, calendar, actor, revision=1, action='publish', value=None, reason=None, operation=None, overrides=None):
        value = ORACLE.minimum() if value is None else value
        raw = raw_values(value)
        digest = sha256(raw).digest()
        operation = operation or str(uuid4())
        receipt = ORACLE.submission(actor[0], operation, calendar, action, revision - 1, digest.hex(), reason)
        fields = dict(calendar_id=literal(calendar), revision=str(revision), values_canonical=blob(raw),
            values_digest=blob(digest), operation_id=literal(operation), action=literal(action),
            reason='NULL' if reason is None else literal(reason), submission_canonical=blob(receipt),
            submission_digest=blob(sha256(receipt).digest()), recorded_at_seconds='1789530000',
            recorded_at_nanoseconds='1234', recorded_by=literal(actor[0]), recorded_by_email=literal(actor[1]))
        fields.update(overrides or {})
        return 'INSERT INTO judicial_calendar_revisions(' + ','.join(fields) + ') VALUES (' + ','.join(fields.values()) + ')'

    def publish(self, calendar=None, actor=None, value=None, operation=None):
        calendar = calendar or str(uuid4())
        actor = actor or self.actor()
        self.sql('BEGIN; INSERT INTO judicial_calendars(id) VALUES (' + literal(calendar) + ');'
                 + self.insert(calendar, actor, value=value, operation=operation) + ';COMMIT;')
        return calendar, actor
