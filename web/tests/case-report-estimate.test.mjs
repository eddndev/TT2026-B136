import test from 'node:test';
import assert from 'node:assert/strict';
import { reportValue } from '../src/lib/case-report-values.mjs';
import * as presentation from '../src/lib/case-reports-presentation.mjs';
import { report, pending } from './browser/case-reports-fixtures.mjs';

const outcomes = ['pending', 'accepted', 'rejected', 'uncertain', 'access_revoked'];

test('legacy report responses remain valid with unknown estimate and email', () => {
  const legacy = report();
  assert.equal(reportValue(legacy), legacy);
  const explicit = pending({ estimated_total_ms: null, ready_email: null });
  assert.equal(reportValue(explicit), explicit);
});

test('report metadata accepts exact positive safe milliseconds and public email outcomes', () => {
  for (const estimated_total_ms of [1, 2_000, 2_001, 65_000, Number.MAX_SAFE_INTEGER]) {
    for (const ready_email of [null, ...outcomes]) {
      const value = report({ estimated_total_ms, ready_email });
      assert.equal(reportValue(value), value);
    }
  }
});

test('report metadata rejects malformed estimates and invented email outcomes', () => {
  for (const estimated_total_ms of [
    0,
    -1,
    1.5,
    NaN,
    Infinity,
    Number.MAX_SAFE_INTEGER + 1,
    '2001',
    true,
    {},
    undefined,
  ]) {
    assert.throws(() => reportValue(report({ estimated_total_ms })));
  }
  for (const ready_email of ['sent', 'delivered', 'Accepted', '', 1, false, {}, undefined]) {
    assert.throws(() => reportValue(report({ ready_email })));
  }
});

test('initial estimate includes waiting and never substitutes the observed duration', () => {
  const queued = pending({ estimated_total_ms: 65_000, ready_email: null });
  assert.equal(presentation.reportEstimate(queued), '1 min 5 s');
  assert.equal(presentation.reportDuration(queued), null);
  const ready = report({ estimated_total_ms: 65_000, ready_email: 'pending' });
  assert.equal(presentation.reportEstimate(ready), '1 min 5 s');
  assert.equal(presentation.reportDuration(ready), '1 min');
  ready.updated_at = '2026-09-27T12:05:00Z';
  ready.notice.read_at = ready.updated_at;
  assert.equal(presentation.reportEstimate(ready), '1 min 5 s');
  assert.equal(presentation.reportDuration(ready), '1 min');
});

test('unavailable estimates stay unknown and partial seconds round upward', () => {
  for (const value of [
    report(),
    pending(),
    report({ estimated_total_ms: null, ready_email: null }),
  ]) {
    assert.equal(presentation.reportEstimate(value), null);
  }
  assert.equal(presentation.reportEstimate(pending({ estimated_total_ms: 1 })), '1 s');
  assert.equal(presentation.reportEstimate(pending({ estimated_total_ms: 2_001 })), '3 s');
});

test('email presentation distinguishes provider acceptance from mailbox delivery', () => {
  const labels = outcomes.map((ready_email) =>
    presentation.reportEmailStatus(report({ ready_email })),
  );
  assert.equal(new Set(labels).size, outcomes.length);
  assert.match(labels[0], /pendiente/i);
  assert.match(labels[1], /aceptado por el proveedor/i);
  assert.match(labels[1], /no confirma entrega en la bandeja/i);
  assert.match(labels[2], /rechaz/i);
  assert.match(labels[3], /no se pudo confirmar/i);
  assert.match(labels[4], /cambio de acceso/i);
  for (const label of [labels[0], labels[2], labels[3], labels[4]]) {
    assert.doesNotMatch(label, /enviado|entregado|recibir\u00e1s/i);
  }
  assert.match(presentation.reportEmailStatus(report()), /sin aviso por correo registrado/i);
  assert.match(
    presentation.reportEmailStatus(report({ ready_email: null })),
    /sin aviso por correo registrado/i,
  );
});
