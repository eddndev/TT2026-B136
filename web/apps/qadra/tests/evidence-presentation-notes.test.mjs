// The notes that frame a validation result: what a correct signature means
// when the result is not complete, the informational current status per
// certificate and the comparison of the pinned roots with the server.
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  currentStatusView,
  serverComparison,
  signatureNote,
} from '../src/lib/evidence/presentation.mjs';

const finding = (outcome, code, detail = '') => ({ outcome, code, detail });
const component = (kind, outcome, findings = []) => ({ kind, outcome, findings });

function report(overall, signature) {
  return {
    overall,
    components: [
      component('package', 'checked'),
      component('document-signature', signature),
      component('certificates', overall === 'checked' ? 'checked' : overall),
    ],
  };
}

test('a correct signature in a result that is not complete is explained', () => {
  assert.equal(signatureNote(report('checked', 'checked')), null);
  assert.equal(signatureNote(report('failed', 'failed')), null);
  assert.equal(signatureNote({ overall: 'not-supported', components: [] }), null);
  for (const overall of ['failed', 'indeterminate', 'not-supported']) {
    const note = signatureNote(report(overall, 'checked'));
    assert.match(note, /firma.*correcta/i, overall);
    assert.match(
      note,
      /no significa por s\u00ed solo que el documento se haya modificado/,
      overall,
    );
    assert.match(note, /no se puede confirmar qui\u00e9n lo firm\u00f3/, overall);
  }
});

test('the current status names each certificate and reads at the present', () => {
  const rows = currentStatusView({
    current_status: [
      finding('checked', 'current.instant', '1793145600, read from the client clock'),
      finding('checked', 'current.within_validity', 'server seal signer'),
      finding('failed', 'current.outside_validity', 'server seal root: valid 1..=2, now 3'),
      finding('checked', 'crl.not_revoked', 'server seal signer (served CRL)'),
      finding('failed', 'crl.revoked', 'holder: 2026 (package CRL)'),
      finding('indeterminate', 'crl.not_covering', 'time-stamping certificate (package CRL)'),
      finding('indeterminate', 'current.no_crl', 'holder root: no CRL from its issuer'),
    ],
  });
  assert.deepEqual(
    rows.map(({ subject, message, tone }) => [subject, message, tone]),
    [
      ['Certificado del sello del servidor', 'Sigue vigente hoy.', 'success'],
      ['Ra\u00edz del sello del servidor', 'Ya no est\u00e1 vigente hoy.', 'danger'],
      ['Certificado del sello del servidor', 'No est\u00e1 revocado hoy.', 'success'],
      ['Certificado de quien firm\u00f3', 'Est\u00e1 revocado hoy.', 'danger'],
      [
        'Certificado de la autoridad del sello de tiempo',
        'No hay una lista de revocaci\u00f3n vigente para hoy.',
        'warning',
      ],
      [
        'Ra\u00edz de la firma individual',
        'No hay una lista de revocaci\u00f3n de quien la emiti\u00f3.',
        'warning',
      ],
    ],
  );
});

test('the comparison with the server says when the pinned roots agree', () => {
  assert.deepEqual(serverComparison({ trust: [] }), [
    'Las ra\u00edces fijas de esta versi\u00f3n de Qadra coinciden con las que usa el servidor.',
  ]);
  const notes = serverComparison({
    trust: [
      { code: 'trust.pinned_not_served', purpose: 'server-seal', fingerprint: 'a', detail: '' },
      { code: 'trust.pinned_not_served', purpose: 'time-stamping', fingerprint: 'a', detail: '' },
    ],
  });
  assert.deepEqual(notes, [
    'La ra\u00edz fija con huella a (Sello del servidor, Sello de tiempo) ya no aparece en la confianza que usa el servidor.',
  ]);
  const served = serverComparison({
    trust: [
      {
        code: 'trust.served_not_pinned',
        purpose: 'individual-signing',
        fingerprint: '0123456789abcdef',
        detail: '',
      },
    ],
  });
  assert.deepEqual(served, [
    'El servidor usa la ra\u00edz con huella 01234567 (Firma individual), que no est\u00e1 en la lista fija de Qadra.',
  ]);
  const token = serverComparison({
    trust: [
      {
        code: 'trust.token_anchor_not_pinned',
        purpose: 'time-stamping',
        fingerprint: 'b',
        detail: '',
      },
    ],
  });
  assert.deepEqual(token, [
    'Las ra\u00edces fijas de esta versi\u00f3n de Qadra coinciden con las que usa el servidor.',
  ]);
  const unavailable = serverComparison({
    trust: [{ code: 'trust.route_unavailable', purpose: null, fingerprint: '', detail: '' }],
  });
  assert.equal(unavailable.length, 1);
  assert.match(unavailable[0], /^No se pudo consultar la confianza actual del servidor/);
});
