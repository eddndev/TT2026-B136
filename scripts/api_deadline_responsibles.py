"""Verify authorized responsible selection using synthetic real HTTP accounts."""
from uuid import UUID
from api_deadlines_support import TOKEN, request


def candidates(base, case_id, token=TOKEN, suffix='?limit=100'):
    page = request('GET', base + '/responsibles' + suffix, token=token)
    assert set(page) == {'case_id', 'responsibles', 'has_more', 'next_after_id'}
    assert page['case_id'] == case_id
    rows = page['responsibles']
    assert all(set(row) == {'id', 'email', 'role'} for row in rows)
    assert all(row['email'].strip() and row['role'] in ['owner', 'litigator', 'paralegal'] for row in rows)
    ids = [row['id'] for row in rows]
    assert ids == sorted(set(ids), key=lambda value: UUID(value).int)
    return page


def verify_selector(route, foreign, users, tokens):
    base = route + '/deadlines'
    case_id = route.rsplit('/', 1)[1]
    owner = request('GET', '/api/v1/auth/me')
    assert owner['role'] == 'owner'
    # The new synthetic case initially assigns its creator. Owner access is global.
    request('DELETE', route + '/members/' + owner['id'], expected=204)
    page = candidates(base, case_id)
    assert not page['has_more'] and page['next_after_id'] is None
    rows = page['responsibles']
    assert sum(row['id'] == owner['id'] for row in rows) == 1
    staff = {row['id']: row['role'] for row in rows if row['role'] != 'owner'}
    assert staff == {users[role]: role for role in ['litigator', 'paralegal']}
    assert users['client'] not in {row['id'] for row in rows}
    for role in ['litigator', 'paralegal']:
        assert candidates(base, case_id, tokens[role]) == page
    request('GET', base + '/responsibles', token=tokens['client'], expected=403)
    foreign_case = foreign.split('/')[-2]
    other = candidates(foreign, foreign_case)
    assert all(row['role'] == 'owner' for row in other['responsibles'])
    request('GET', foreign + '/responsibles', token=tokens['litigator'], expected=404)
    nil = '00000000-0000-0000-0000-000000000000'
    assert candidates(base, case_id, suffix='?limit=100&after_id=' + nil) == page
    first = candidates(base, case_id, suffix='?limit=1')
    assert first['responsibles'] == rows[:1] and first['has_more']
    assert first['next_after_id'] == rows[0]['id']
    rest = candidates(base, case_id, suffix='?limit=100&after_id=' + first['next_after_id'])
    assert rest['responsibles'] == rows[1:] and not rest['has_more']
    empty = candidates(base, case_id, suffix='?after_id=ffffffff-ffff-ffff-ffff-ffffffffffff')
    assert empty['responsibles'] == [] and not empty['has_more'] and empty['next_after_id'] is None
    for query in ['scope=all', 'limit=0', 'limit=101', 'limit=1&limit=2', 'after_id=null',
                  'after_id=' + nil + '&after_id=' + nil]:
        request('GET', base + '/responsibles?' + query, expected=400)
    print('Deadline responsible selector passed: active case staff, unassigned Owner, Client denial, pagination and strict queries.')


def verify_revocation(route, users, tokens):
    base = route + '/deadlines'
    case_id = route.rsplit('/', 1)[1]
    page = candidates(base, case_id, tokens['paralegal'])
    ids = {row['id'] for row in page['responsibles']}
    assert users['litigator'] not in ids and users['client'] not in ids
    assert users['paralegal'] in ids
    request('GET', base + '/responsibles', token=tokens['litigator'], expected=404)
    print('Deadline responsible revocation passed: removed membership disappears before another selection.')
