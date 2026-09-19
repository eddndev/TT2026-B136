"""Helpers restricted to api-demo.sh's disposable content fixture database."""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import subprocess
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen
from uuid import UUID
from api_procedural_facts_support import BASE, TOKEN, WORK, request

STATE = WORK / 'document-content-api-state.json'
INBOX = '/api/v1/document-integrity-incidents'


def enroll(role, route):
    email = 'document-content-' + role + '@example.com'
    password = 'synthetic content fixture password'
    enrollment = request('POST', '/api/v1/users', {
        'email': email, 'password': password, 'role': role,
    }, 201)
    challenge = request('POST', '/api/v1/auth/login', {'email': email, 'password': password})
    session = request('POST', '/api/v1/auth/mfa/recovery', {
        'challenge_token': challenge['challenge_token'], 'code': enrollment['recovery_codes'][0],
    })
    user = enrollment['user']['id']
    request('PUT', route + '/members/' + user, expected=204)
    return user, session['access_token']


def content_path(route, row):
    return route + '/documents/' + row['id'] + '/versions/' + str(row['version']) + '/content'


def content(path, row, payload, token=TOKEN, expected=200, code=None):
    query = Request(BASE + path, headers={'Authorization': 'Bearer ' + token})
    try:
        response = urlopen(query, timeout=60)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(16 * 1024 * 1024 + 1)
        assert len(raw) <= 16 * 1024 * 1024, 'Content response exceeds fixture budget'
        assert response.status == expected, (path, response.status, expected)
        if expected != 200:
            assert response.headers.get_content_type() == 'application/json'
            assert response.headers.get('Content-Disposition') is None
            assert response.headers.get('X-Document-Digest') is None
            assert payload not in raw, 'Rejected content must not return document bytes'
            result = json.loads(raw)
            assert result['error']['code'] == code, (path, result['error']['code'], code)
            return result
        assert raw == payload, 'Exact content bytes differ'
        assert response.headers.get_content_type() == 'application/octet-stream'
        assert response.headers['Content-Disposition'].startswith('attachment; filename="')
        assert response.headers['Cache-Control'] == 'no-store'
        assert response.headers['X-Content-Type-Options'] == 'nosniff'
        assert response.headers['Content-Length'] == str(len(payload))
        assert response.headers['X-Case-Id'] == row['case_id']
        assert response.headers['X-Document-Id'] == row['id']
        assert response.headers['X-Document-Version'] == str(row['version'])
        assert response.headers['X-Document-Digest'] == row['digest'] == hashlib.sha256(raw).hexdigest()
    return None


def sql(statement):
    # SQL is internal fixture text; UUIDs and bytea hex are validated before use.
    environment = dict(os.environ, PGDATABASE=os.environ['TT_CONTENT_ADMIN_URL'])
    result = subprocess.run(['psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1',
                             '-d', os.environ['TT_CONTENT_ADMIN_URL']],
                            input=statement, text=True, capture_output=True,
                            env=environment, timeout=30, check=False)
    if result.returncode:
        raise AssertionError('Disposable content fixture SQL failed; no data was logged')
    return result.stdout.strip()


def require_disposable_database():
    location = urlsplit(os.environ['TT_CONTENT_ADMIN_URL'])
    assert location.scheme == 'postgresql' and location.hostname == '127.0.0.1'
    assert location.path == '/imported' and location.port == int(os.environ['TT_CONTENT_PG_PORT'])
    database, directory, superuser = sql(
        "SELECT current_database(), current_setting('data_directory'), "
        "current_setting('is_superuser')").split('|')
    assert database == 'imported' and superuser == 'on'
    assert Path(directory).resolve() == Path(os.environ['TT_CONTENT_PG_DATA']).resolve()
    assert Path(directory).resolve().parent == WORK.resolve()


def document_filter(row):
    identifier, case = str(UUID(row['id'])), str(UUID(row['case_id']))
    version = int(row['version'])
    assert version > 0
    return f"id='{identifier}' AND case_id='{case}' AND version={version}"


def counts(row):
    identifier = str(UUID(row['id']))
    version = int(row['version'])
    prefix = f"%:document:{identifier}:version:{version}:%"
    return json.loads(sql(
        "SELECT json_build_object('authorized',(SELECT count(*) FROM audit_events "
        f"WHERE action='document.content_authorized' AND resource LIKE '{prefix}'),"
        "'rejected',(SELECT count(*) FROM audit_events WHERE action='document.content_rejected'),"
        "'incidents',(SELECT count(*) FROM document_integrity_incidents))"))


@contextmanager
def corrupted_ciphertext(row):
    # Deliberate fixture-only corruption bypasses immutable-row triggers in one
    # local transaction. This is not a production repair or recovery procedure.
    require_disposable_database()
    where = document_filter(row)
    original = sql(f"SELECT encode(vault,'hex') FROM documents WHERE {where}")
    ciphertext = bytearray.fromhex(original)
    assert len(ciphertext) > 97 and ciphertext.hex() == original
    ciphertext[-1] ^= 1

    def replace(value):
        assert bytes.fromhex(value).hex() == value
        changed = sql("BEGIN; SET LOCAL session_replication_role=replica; "
                      f"UPDATE documents SET vault=decode('{value}','hex') WHERE {where} RETURNING id; "
                      "COMMIT;")
        assert changed == row['id']

    try:
        replace(ciphertext.hex())
        yield
    finally:
        # Even a failed HTTP assertion restores the exact bytes before backup.
        # SET LOCAL ends with each transaction; no trigger is left disabled.
        replace(original)
        assert sql(f"SELECT encode(vault,'hex') FROM documents WHERE {where}") == original


def incident_pages():
    path, records, seen = INBOX + '?limit=1', [], set()
    for _ in range(100):
        page = request('GET', path)
        assert len(page['incidents']) <= 1
        for row in page['incidents']:
            assert row['id'] not in seen
            seen.add(row['id'])
            records.append(row)
        if not page['has_more']:
            assert page['next_after_id'] is None
            assert [row['id'] for row in records] == sorted(seen)
            return records
        assert page['incidents'] and page['next_after_id'] == page['incidents'][-1]['id']
        path = INBOX + '?limit=1&after_id=' + page['next_after_id']
    raise AssertionError('Incident fixture exceeded its bounded page budget')
