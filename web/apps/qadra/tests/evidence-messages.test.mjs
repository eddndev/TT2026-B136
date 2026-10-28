// Every finding code the evidence engine can emit has a Spanish message, and
// the texts of the evidence validator stay ASCII without long dashes.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  findingMessages,
  findingMessage,
  FAMILY_FALLBACKS,
  HISTORICAL_NOTE,
} from '../src/lib/evidence/messages.mjs';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const engine = join(root, 'crates/evidence-verify/src');
const app = fileURLToPath(new URL('../', import.meta.url));
const FILE_EXTENSIONS = new Set(['pem', 'json', 'tsr', 'der', 'sig', 'md', 'zip']);

function sources(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    if (statSync(path).isDirectory()) return name === 'tests' ? [] : sources(path);
    const test = name === 'tests.rs' || name.endsWith('_tests.rs');
    return name.endsWith('.rs') && !test ? [path] : [];
  });
}

function engineCodes() {
  const codes = new Set();
  for (const path of sources(engine)) {
    const text = readFileSync(path, 'utf8');
    for (const [, code] of text.matchAll(/"([a-z][a-z0-9_]*(?:\.[a-z0-9_]+)+)"/g)) {
      if (!FILE_EXTENSIONS.has(code.split('.').at(-1))) codes.add(code);
    }
  }
  return [...codes].sort();
}

test('the engine sources yield the finding codes and no entry names', () => {
  const codes = engineCodes();
  assert.ok(codes.length > 150, `only ${codes.length} codes found`);
  for (const code of [
    'zip.admitted',
    'signature.valid',
    'trust.pinned_not_served',
    'current.no_crl',
  ])
    assert.ok(codes.includes(code), code);
  for (const name of ['ca.pem', 'crl.pem', 'manifiesto.json', 'manifiesto.json.tsr'])
    assert.ok(!codes.includes(name), name);
});

test('every finding code of the engine has its own message', () => {
  const missing = engineCodes().filter((code) => !Object.hasOwn(findingMessages, code));
  assert.deepEqual(missing, []);
  for (const code of engineCodes()) {
    const text = findingMessages[code];
    assert.equal(typeof text, 'string', code);
    assert.ok(text.trim().length > 8, code);
    assert.equal(findingMessage(code), text);
  }
});

test('the message table holds no code the engine does not emit', () => {
  const codes = new Set(engineCodes());
  assert.deepEqual(
    Object.keys(findingMessages).filter((code) => !codes.has(code)),
    [],
  );
});

test('the special rows carry the agreed plain texts', () => {
  assert.match(findingMessages['zip.method'], /no es el paquete que export\u00f3 Qadra/);
  assert.match(findingMessages['zip.method'], /descarga de nuevo la evidencia/);
  assert.equal(
    findingMessages['zip.crc_mismatch'],
    'Una parte del ZIP no coincide con el control que el propio ZIP guarda: el paquete se modific\u00f3 o se da\u00f1\u00f3 despu\u00e9s de exportarse.',
  );
  for (const code of ['profile.unknown_entry', 'profile.missing_entry'])
    assert.equal(findingMessages[code], 'El paquete tiene archivos de m\u00e1s o le faltan.');
  assert.match(findingMessages['signature.valid'], /firma.*correcta/i);
  assert.match(findingMessages['signature.invalid'], /firma.*incorrecta/i);
  assert.equal(
    findingMessages['cert.profile.key_size'],
    'El certificado no cumple el perfil estricto.',
  );
  for (const code of ['trust.root_not_pinned', 'trust.served_not_pinned'])
    assert.match(findingMessages[code], /^La ra\u00edz no est\u00e1 en la lista fija de Qadra/);
  assert.equal(
    findingMessages['trust.pinned_not_served'],
    'La ra\u00edz sigue fija pero el servidor ya no la usa.',
  );
  for (const code of ['revocation.certificate_unavailable', 'crl.not_covering'])
    assert.match(findingMessages[code], /^Falta la lista de revocaci\u00f3n que cubre esa fecha/);
  assert.match(HISTORICAL_NOTE, /paquetes hist\u00f3ricos/);
});

test('an unknown code falls back to the message of its family', () => {
  for (const family of [
    'der',
    'pem',
    'json',
    'individual_json',
    'statement',
    'statements',
    'token',
    'context',
    'binding',
    'consent',
  ]) {
    assert.ok(FAMILY_FALLBACKS[family], family);
    assert.equal(findingMessage(`${family}.not_a_real_code`), FAMILY_FALLBACKS[family]);
  }
  assert.match(findingMessage('zip.not_a_real_code'), /no es el paquete que export\u00f3 Qadra/);
  assert.match(findingMessage('unknown'), /\S/);
});

test('validator sources are ASCII and have no long dashes', () => {
  const files = [
    ...readdirSync(join(app, 'src/lib/evidence')).map((name) =>
      join(app, 'src/lib/evidence', name),
    ),
    ...readdirSync(join(app, 'src/components'))
      .filter((name) => name.startsWith('EvidenceValidation'))
      .map((name) => join(app, 'src/components', name)),
    join(app, 'src/styles/evidence-validation.css'),
  ];
  assert.ok(files.some((path) => path.endsWith('EvidenceValidation.svelte')));
  for (const path of files) {
    const text = readFileSync(path, 'utf8');
    assert.ok(/^[\x00-\x7f]*$/.test(text), `${path} is not ASCII`);
    assert.ok(!/\\u(?:2013|2014)|&[mn]dash;|&#821[12];/i.test(text), `${path} has a long dash`);
  }
  for (const text of [...Object.values(findingMessages), ...Object.values(FAMILY_FALLBACKS)])
    assert.ok(!/[\u2013\u2014]/.test(text));
});
