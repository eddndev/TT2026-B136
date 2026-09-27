"""Accept supported upload bytes and preserve their exact restored content."""
import base64
import hashlib
from pathlib import Path
from api_document_content_support import content, content_path, request


def capture():
    root = Path(__file__).resolve().parents[1]
    media = root / 'crates/infrastructure/tests/fixtures/media-admission'
    inputs = [
        ('pdf', (root / 'crates/infrastructure/tests/fixtures/stage-support.pdf').read_bytes()),
        ('docx', (root / 'crates/infrastructure/src/document_formats/docx/tests/fixtures/producer.docx').read_bytes()),
        ('txt', b'Original UTF-8 text: \xc3\xb1\r\n'),
        *[(extension, (media / f'tiny.{extension}').read_bytes())
          for extension in ['jpg', 'png', 'mp3', 'wav', 'mp4']],
    ]
    record = request('POST', '/api/v1/cases', {
        'title': 'Document format admission', 'reference': 'FORMAT-ADMISSION',
    }, 201)
    route = '/api/v1/cases/' + record['id']
    collection = route + '/documents'
    saved = []
    for extension, payload in inputs:
        row = request('POST', collection, payload, 201,
                      headers={'X-Document-Name': f'admitted.{extension}'})
        assert row['version'] == 1 and row['sealed'] is False
        assert row['digest'] == hashlib.sha256(payload).hexdigest()
        content(content_path(route, row), row, payload)
        saved.append({'row': row, 'payload': base64.b64encode(payload).decode('ascii')})
    before = request('GET', collection)
    for payload, code in [(b'GIF89a\x00', 'document_format_unsupported'),
                          (inputs[4][1][:-1], 'document_format_invalid')]:
        request('POST', collection, payload, 422, code=code,
                headers={'X-Document-Name': 'misleading.pdf'})
        assert request('GET', collection) == before
    first = saved[0]['row']
    path = collection + '/' + first['id']
    history = request('GET', path + '/versions')
    request('POST', path + '/versions?expected_version=1', b'GIF89a\x00', 422,
            code='document_format_unsupported', headers={'X-Document-Name': 'rejected.pdf'})
    assert request('GET', path) == first
    assert request('GET', path + '/versions') == history
    print('Format admission passed: eight formats, exact bytes and mutation-free rejection.')
    return {'route': route, 'documents': saved}


def restore(saved):
    for entry in saved['documents']:
        row = entry['row']
        path = saved['route'] + '/documents/' + row['id']
        assert request('GET', path) == row
        content(content_path(saved['route'], row), row, base64.b64decode(entry['payload']))
    print('Format restore passed: all eight immutable upload contents survived.')
