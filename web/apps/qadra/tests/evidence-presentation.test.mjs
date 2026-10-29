import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  MAX_EVIDENCE_FILE_BYTES,
  appliedRoots,
  componentView,
  instantView,
  isNotQadraExport,
  overallLabel,
  trustNotes,
} from '../src/lib/evidence/presentation.mjs';

const finding = (outcome, code, detail = '') => ({ outcome, code, detail });
const component = (kind, outcome, findings = []) => ({ kind, outcome, findings });

test('the four overall results have plain labels', () => {
  assert.equal(overallLabel({ overall: 'checked' }).label, 'Completo');
  assert.equal(overallLabel({ overall: 'indeterminate' }).label, 'Parcial');
  assert.equal(overallLabel({ overall: 'failed' }).label, 'Fallido');
  assert.equal(overallLabel({ overall: 'not-supported' }).label, 'No compatible');
  const tones = ['checked', 'indeterminate', 'failed', 'not-supported'].map(
    (overall) => overallLabel({ overall }).tone,
  );
  assert.deepEqual(tones, ['success', 'warning', 'danger', 'neutral']);
  for (const overall of ['checked', 'indeterminate', 'failed', 'not-supported'])
    assert.ok(overallLabel({ overall }).summary.length > 10);
  assert.equal(overallLabel({ overall: 'other' }).label, 'No compatible');
});

test('the document signature reads as correct or incorrect', () => {
  const valid = componentView(
    component('document-signature', 'checked', [finding('checked', 'signature.valid')]),
  );
  assert.equal(valid.title, 'Firma del documento');
  assert.equal(valid.status, 'Firma correcta');
  const invalid = componentView(
    component('document-signature', 'failed', [finding('failed', 'signature.invalid')]),
  );
  assert.equal(invalid.status, 'Firma incorrecta');
  assert.equal(invalid.tone, 'danger');
  assert.deepEqual(invalid.messages, ['La firma del documento es incorrecta.']);
});

test('the strict certificate profile is kept apart from the trust of its root', () => {
  const strict = componentView(
    component('certificates', 'not-supported', [
      finding('checked', 'cert.profile', 'certificado.pem: Signer'),
      finding('checked', 'cert.profile', 'ca.pem: Root'),
      finding('not-supported', 'trust.root_not_pinned', 'ca.pem: abc is not pinned'),
    ]),
  );
  assert.equal(strict.profile, 'Cumple el perfil estricto');
  assert.equal(strict.trust, 'La ra\u00edz no est\u00e1 en la lista fija de Qadra');
  assert.equal(strict.status, 'No compatible');
  const loose = componentView(
    component('certificates', 'failed', [
      finding('failed', 'cert.profile.key_usage', 'certificado.pem'),
      finding('checked', 'trust.root_pinned', 'ca.pem: Qadra for server-seal'),
    ]),
  );
  assert.equal(loose.profile, 'No cumple el perfil estricto');
  assert.equal(loose.trust, 'Ra\u00edz fija en Qadra');
  const served = componentView(
    component('certificates', 'indeterminate', [
      finding('checked', 'cert.profile', 'certificado.pem: Signer'),
      finding('indeterminate', 'trust.root_served_not_pinned', 'ca.pem'),
    ]),
  );
  assert.equal(served.trust, 'Confianza sin confirmar');
  assert.equal(served.status, 'Falta evidencia para decidirlo');
  assert.equal(componentView(component('document-signature', 'checked')).profile, null);
});

test('missing evidence is not reported as a failure', () => {
  const view = componentView(
    component('revocation', 'indeterminate', [
      finding('indeterminate', 'crl.not_covering', 'crl.pem'),
      finding('indeterminate', 'crl.not_covering', 'tsa'),
    ]),
  );
  assert.equal(view.status, 'Falta evidencia para decidirlo');
  assert.equal(view.tone, 'warning');
  assert.deepEqual(view.messages, ['Falta la lista de revocaci\u00f3n que cubre esa fecha.']);
  const skipped = componentView(component('manifest', 'not-applicable'));
  assert.equal(skipped.status, 'No aplica');
  assert.deepEqual(skipped.messages, []);
});

test('instants name their source and are written in UTC', () => {
  const token = instantView({ label: 'document token genTime', unix: 1793145600, source: 'token' });
  assert.equal(token.label, 'Fecha del sello de tiempo del documento');
  assert.equal(token.when, '2026-10-28 00:00:00 UTC');
  assert.equal(token.source, 'Sello de tiempo verificado');
  assert.equal(
    instantView({ label: 'accepted_at', unix: 0, source: 'application-record' }).source,
    'Registro de Qadra sin sello de tiempo',
  );
  const now = instantView({ label: 'current status', unix: 1, source: 'caller' });
  assert.equal(now.label, 'Momento de esta revisi\u00f3n');
  assert.equal(now.source, 'Reloj de este equipo');
  assert.equal(instantView({ label: 'other', unix: 1, source: 'x' }).label, 'Otra fecha');
});

test('the manifest token and the receipt have their own labels', () => {
  const manifest = instantView({ label: 'manifest token genTime', unix: 1, source: 'token' });
  assert.equal(manifest.label, 'Fecha del sello de tiempo del manifiesto');
  assert.equal(manifest.source, 'Sello de tiempo verificado');
  const receipt = instantView({
    label: 'receipt accepted_at',
    unix: 1793145600,
    source: 'application-record',
  });
  assert.equal(receipt.label, 'Aceptaci\u00f3n de la firma, seg\u00fan el recibo');
  assert.equal(receipt.when, '2026-10-28 00:00:00 UTC');
  assert.equal(receipt.source, 'Registro de Qadra sin sello de tiempo');
  const failed = componentView(
    component('manifest', 'failed', [
      finding('checked', 'token.verified'),
      finding('failed', 'manifest.artifact_mismatch', 'acta.pdf differs'),
      finding('failed', 'receipt.seconds_mismatch', 'the receipt states 1'),
    ]),
  );
  assert.equal(failed.title, 'Manifiesto y su sello de tiempo');
  assert.equal(failed.status, 'Fallido');
  assert.equal(failed.messages.length, 2);
  for (const text of failed.messages) assert.doesNotMatch(text, /no sabe describir/);
  const checked = componentView(
    component('manifest', 'checked', [finding('checked', 'token.verified')]),
  );
  assert.equal(checked.status, 'Correcto');
  assert.deepEqual(checked.messages, []);
});

test('a ZIP that Qadra did not export is told apart from a damaged one', () => {
  const report = (findings) => ({
    overall: 'not-supported',
    components: [component('package', 'not-supported', findings)],
  });
  assert.equal(isNotQadraExport(report([finding('not-supported', 'zip.method', 'a')])), true);
  assert.equal(isNotQadraExport(report([finding('not-supported', 'profile.unrecognized')])), false);
  assert.equal(
    isNotQadraExport({
      overall: 'failed',
      components: [component('package', 'failed', [finding('failed', 'zip.crc_mismatch')])],
    }),
    false,
  );
  assert.equal(isNotQadraExport({ overall: 'checked', components: [] }), false);
});

test('the applied roots and the trust notes come from the report', () => {
  const pinned = [
    { fingerprint: 'a'.repeat(64), purposes: ['server-seal'], label: 'Qadra' },
    { fingerprint: 'b'.repeat(64), purposes: ['time-stamping'], label: 'Otra' },
  ];
  const report = {
    components: [
      component('certificates', 'checked', [
        finding('checked', 'trust.root_pinned', 'ca.pem: Qadra for server-seal'),
      ]),
      component('document-token', 'checked', [
        finding('checked', 'token.anchor_pinned', 'Qadra for time-stamping'),
      ]),
    ],
    trust: [
      { code: 'trust.pinned_not_served', purpose: 'server-seal', fingerprint: 'a', detail: '' },
      { code: 'trust.route_unavailable', purpose: null, fingerprint: '', detail: '' },
    ],
  };
  assert.deepEqual(appliedRoots(report, pinned), [pinned[0]]);
  assert.deepEqual(
    trustNotes(report).map((note) => note.message),
    [
      'La ra\u00edz sigue fija pero el servidor ya no la usa.',
      'No se pudo consultar la confianza actual del servidor. El resultado no cambia; solo el estado actual usa las listas del paquete.',
    ],
  );
});

test('the file limit admits every package the engine accepts', () => {
  const limits = readFileSync(
    new URL('../../../../crates/evidence-verify/src/archive/limits.rs', import.meta.url),
    'utf8',
  );
  const total = /MAX_TOTAL: u64 = (\d+) \* MIB;/.exec(limits);
  assert.ok(total, 'MAX_TOTAL is written in MiB');
  assert.equal(MAX_EVIDENCE_FILE_BYTES, 26 * 1024 * 1024);
  assert.ok(MAX_EVIDENCE_FILE_BYTES >= Number(total[1]) * 1024 * 1024);
});
