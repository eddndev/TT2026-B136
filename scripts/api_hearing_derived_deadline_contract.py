"""Assert HTTP evidence links without pretending to recompute canonical hashes."""
from datetime import datetime
import re


def digest(value):
    assert isinstance(value, str) and re.fullmatch(r'[0-9a-f]{64}', value)


def recorded_instant(value):
    match = re.fullmatch(
        r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?(Z|[+-]\d\d:\d\d)', value)
    assert match, 'Expected an exact RFC3339 capture instant'
    base, fraction, zone = match.groups()
    at = datetime.fromisoformat(base + zone.replace('Z', '+00:00'))
    return {'unix_seconds': int(at.timestamp()),
            'nanosecond': int((fraction or '').ljust(9, '0')),
            'offset_seconds': int(at.utcoffset().total_seconds())}


def assert_creation(ready, record, actor):
    """Bind the returned pair and origin to the reviewed prospective command."""
    assert ready['state'] == 'ready'
    command = ready['command']
    result_command, deadline_command = command['result'], command['deadline']
    result, deadline, origin = record['result'], record['deadline'], record['origin']
    assert record['case_id'] == result['case_id'] == deadline['case_id'] == command['case_id']
    assert record['command'] == command
    assert ready['result']['actor_id'] == actor['id']
    assert record['review_digest'] == ready['review_digest']
    for key in ['review_digest', 'capture_digest']:
        digest(record[key])
    assert result['id'] == result_command['result_id']
    assert result['hearing_id'] == result_command['hearing_id']
    assert deadline['id'] == deadline_command['deadline_id']
    assert result['revision'] == deadline['revision'] == 1
    assert result['status'] == 'recorded'
    assert deadline['status'] == 'active'
    for key in ['values', 'values_digest', 'anchor', 'continuation', 'attendees', 'support']:
        assert result[key] == ready['result'][key], key
    assert result['values'] == result_command['change']['values']
    administration = ready['result']['observed_administration']
    assert result['recorded_administration_revision'] == administration['revision']
    assert result['recorded_administration_digest'] == administration['values_digest']
    assert result['receipt'] == {
        'operation_id': result_command['operation_id'], 'action': 'record',
        'expected_revision': 0, 'submission_digest': ready['result']['submission_digest']}
    assert result['recorded_by'] == {key: actor[key] for key in ['id', 'email']}
    assert deadline['recorded_by'] == {'kind': 'user', **result['recorded_by']}
    assert deadline['recorded_at'] == recorded_instant(result['recorded_at'])
    assert deadline['definition'] == ready['deadline']['definition'] == deadline_command['change']['definition']
    assert deadline['responsible'] == ready['deadline']['responsible']
    assert deadline['tracking']['policies'] == ready['deadline']['tracking'] == deadline_command['change']['tracking']
    assert deadline['calculation']['result'] == ready['deadline']['result']
    receipt = deadline['receipt']
    assert receipt['version']['kind'] == 'v2'
    assert receipt['operation_id'] == deadline_command['operation_id']
    assert receipt['action'] == 'register' and receipt['expected_revision'] == 0
    for key in ['submission_digest', 'review_digest', 'capture_digest']:
        digest(receipt[key])
    source = deadline['calculation']['material']['source']
    assert source['case_id'] == command['case_id']
    assert source['reference'] == deadline['definition']['input']['selection']['source']['value']
    assert source['values_digest'] == result['values_digest']
    assert source['submission_digest'] == result['receipt']['submission_digest']
    observations = [entry for entry in deadline['tracking']['observations']['entries']
                    if entry['role'] == 'source']
    assert len(observations) == 1
    for key, value in {
        'family': 'hearing_result', 'id': result['id'], 'revision': 1,
        'case_id': command['case_id'], 'hearing_id': result['hearing_id'],
        'submission_digest': result['receipt']['submission_digest'],
    }.items():
        assert observations[0][key] == value, key
    event = origin['source_event']
    sequence = event['sequence']
    assert isinstance(sequence, str) and re.fullmatch(r'[1-9][0-9]*', sequence)
    assert 0 < int(sequence) <= 2 ** 63 - 1
    assert event == {'sequence': sequence, 'family': 'hearing_result',
                     'source_id': result['id'], 'revision': 1, 'case_id': command['case_id'],
                     'hearing_id': result['hearing_id'], 'operation_id': result_command['operation_id']}
    assert origin == {
        'case_id': command['case_id'], 'hearing_id': result_command['hearing_id'],
        'result_id': result['id'], 'result_revision': 1,
        'result_operation_id': result_command['operation_id'],
        'deadline_id': deadline['id'], 'deadline_revision': 1,
        'deadline_operation_id': deadline_command['operation_id'], 'recorded_by': actor,
        'review_digest': ready['review_digest'], 'capture_digest': record['capture_digest'],
        'source_event': event,
    }
