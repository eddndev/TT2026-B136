// Runs the generated WebAssembly module in Node on the bilateral contract
// fixtures of crates/evidence-verify/tests/contract-fixtures, packed as the
// server's stored ZIP. It requires the module built by
// bash scripts/build-evidence-verify.sh, as `npm run dev` does.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { findingMessage, FAMILY_FALLBACKS } from '../src/lib/evidence/messages.mjs';
import { profileLabel } from '../src/lib/evidence/presentation.mjs';

const generated = new URL('../src/generated/evidence-verify/', import.meta.url);
const fixtures = new URL(
  '../../../../crates/evidence-verify/tests/contract-fixtures/',
  import.meta.url,
);
const facts = JSON.parse(readFileSync(new URL('fixtures.json', fixtures), 'utf8'));
const SHARED = [
  'contrato.pdf',
  'contrato.pdf.tsr',
  'contrato-solicitud.bin',
  'contrato-manifiesto.json',
  'contrato-manifiesto.json.tsr',
  'contrato-recibo.json',
  'ca.pem',
  'tsa-chain.pem',
];
const PARTY = [
  'declaracion.bin',
  'declaracion.sig',
  'certificado.pem',
  'crl.pem',
  'vinculo.bin',
  'vinculo.sig',
  'vinculo-raiz.der',
  'vinculo-crl.der',
];
const PARTY_1 = [
  ...PARTY.map((name) => `parte-1-${name}`),
  'parte-1-declaracion.sig.tsr',
  'parte-1-tsa-chain.pem',
];
const PARTY_2 = PARTY.map((name) => `parte-2-${name}`);
let validate;

before(async () => {
  const glue = new URL('evidence_verify_wasm.js', generated);
  assert.ok(existsSync(glue), 'Run bash scripts/build-evidence-verify.sh first.');
  const module = await import(glue.href);
  module.initSync({ module: readFileSync(new URL('evidence_verify_wasm_bg.wasm', generated)) });
  validate = module.validate;
});

const CRC_TABLE = Array.from({ length: 256 }, (_, index) => {
  let crc = index;
  for (let bit = 0; bit < 8; bit += 1) crc = crc & 1 ? 0xedb88320 ^ (crc >>> 1) : crc >>> 1;
  return crc >>> 0;
});

function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

// Method 0, flags 0, version needed 10, made by 20, DOS date 1980-01-01 at
// time 0, no extra fields or comments: the layout of the server's writer.
function storedZip(entries) {
  const parts = [];
  const directory = [];
  let offset = 0;
  for (const [name, data] of entries) {
    const fileName = Buffer.from(name, 'ascii');
    const common = Buffer.alloc(26);
    common.writeUInt16LE(10, 0);
    common.writeUInt16LE(0x21, 8);
    common.writeUInt32LE(crc32(data), 10);
    common.writeUInt32LE(data.length, 14);
    common.writeUInt32LE(data.length, 18);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    common.copy(local, 4);
    local.writeUInt16LE(fileName.length, 26);
    parts.push(local, fileName, data);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    common.copy(central, 6);
    central.writeUInt16LE(fileName.length, 28);
    central.writeUInt32LE(offset, 42);
    directory.push(central, fileName);
    offset += local.length + fileName.length + data.length;
  }
  const centralBytes = Buffer.concat(directory);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(centralBytes.length, 12);
  end.writeUInt32LE(offset, 16);
  return new Uint8Array(Buffer.concat([...parts, centralBytes, end]));
}

function pack(names, variant) {
  const entries = names.map((name) => {
    const override = variant && new URL(`variants/${variant}/${name}`, fixtures);
    const file = override && existsSync(override) ? override : new URL(name, fixtures);
    return [name, readFileSync(file)];
  });
  entries.push(['INSTRUCCIONES.md', Buffer.from('# Instrucciones\n')]);
  return storedZip(entries);
}

function policy() {
  const root = readFileSync(new URL('ca.pem', fixtures), 'ascii')
    .replace(/-----[^-]+-----/g, '')
    .replace(/\s+/g, '');
  return JSON.stringify({
    pinned: [
      {
        fingerprint: createHash('sha256').update(Buffer.from(root, 'base64')).digest('hex'),
        purposes: ['server-seal', 'individual-signing', 'time-stamping'],
        label: 'contract fixture root',
      },
    ],
    served: null,
    admitted_policies: ['1.3.6.1.4.1.13762.3'],
    current_instant: facts.instants.tf + 3600,
  });
}

const codes = (report) =>
  report.components.flatMap((component) => component.findings.map((finding) => finding.code));

function assertDescribed(report) {
  const fallbacks = new Set(Object.values(FAMILY_FALLBACKS));
  for (const code of codes(report)) {
    assert.ok(!fallbacks.has(findingMessage(code)), `${code} has no message of its own`);
  }
}

test('the full contract package is checked and labelled as a contract', () => {
  const report = JSON.parse(validate(pack([...SHARED, ...PARTY_1, ...PARTY_2]), policy()));
  assert.equal(report.profile, 'bilateral-contract-v1');
  assert.equal(report.overall, 'checked');
  assert.equal(report.components.length, 8);
  assert.equal(profileLabel(report.profile), 'Contrato firmado por ambas partes');
  assertDescribed(report);
});

test('the partial package is indeterminate and says it proves no contract', () => {
  const names = ['contrato.pdf', 'contrato-solicitud.bin', 'contrato-acuse-parcial.json', 'ca.pem'];
  const report = JSON.parse(validate(pack([...names, ...PARTY_1]), policy()));
  assert.equal(report.profile, 'bilateral-contract-partial-v1');
  assert.equal(report.overall, 'indeterminate');
  assert.equal(profileLabel(report.profile), 'Acuse parcial: no acredita contrato');
  assert.ok(codes(report).includes('contract.partial'));
  assertDescribed(report);
});

test('a firm signature after the request expiry fails with its own message', () => {
  const report = JSON.parse(
    validate(pack([...SHARED, ...PARTY_1, ...PARTY_2], 'expired'), policy()),
  );
  assert.equal(report.overall, 'failed');
  const failed = report.components
    .flatMap((component) => component.findings)
    .filter((finding) => finding.outcome === 'failed')
    .map((finding) => finding.code);
  assert.deepEqual([...new Set(failed)], ['contract.after_expiry']);
  assert.match(findingMessage('contract.after_expiry'), /vigencia de la solicitud/);
  assertDescribed(report);
});
