import test from 'node:test';
import assert from 'node:assert/strict';
import {
  CLIENT_CONSENT_TEXT,
  CLIENT_SIGNED_NOTICE,
  blockerMessages,
  certificateExpired,
  certificateRenewalDue,
  clientSignReady,
  contractDocumentStatus,
  contractable,
  declaredName,
  requestActions,
  underContract,
  verificationPendingText,
} from '../src/lib/contract-presentation.mjs';
import {
  DEFAULT_TTL_DAYS,
  contractReason,
  contractRequestValues,
  contractTitle,
} from '../src/lib/contract-request-draft.mjs';
import { documentStatus } from '../src/lib/workspace.mjs';
import { requestDetail, inputs } from './contract-fixture.mjs';

test('a contracted version is never shown as sealed or pending seal', () => {
  for (const state of ['en_firma', 'firmado']) {
    const status = documentStatus({ sealed: false, contract_state: state });
    assert.equal(status.key, 'contract');
    assert.notEqual(status.label, 'Pendiente de sello');
    assert.notEqual(status.label, 'Sellado');
    assert.equal(underContract({ sealed: false, contract_state: state }), true);
  }
  assert.equal(documentStatus({ sealed: false, contract_state: 'borrador' }).key, 'pending');
  assert.equal(documentStatus({ sealed: false, contract_state: 'abandonado' }).key, 'pending');
  assert.equal(documentStatus({ sealed: true, contract_state: 'firmado' }).key, 'sealed');
  assert.equal(contractDocumentStatus({ sealed: false, contract_state: null }), null);
});

test('the verification tab of a contracted version points to the contract package', () => {
  for (const state of ['en_firma', 'firmado']) {
    const text = verificationPendingText({ sealed: false, contract_state: state });
    assert.match(text, /paquete del contrato/);
    assert.doesNotMatch(text, /sello/);
  }
  assert.match(verificationPendingText({ sealed: false, contract_state: null }), /sello/);
  assert.match(verificationPendingText({ sealed: true }), /Verificar integridad/);
});

test('only an unsealed PDF without a contract offers contract creation to Owner and Litigator', () => {
  const pdf = { sealed: false, contract_state: null, name: 'Contrato.PDF' };
  assert.equal(contractable(pdf, 'owner'), true);
  assert.equal(contractable(pdf, 'litigator'), true);
  assert.equal(contractable(pdf, 'paralegal'), false);
  assert.equal(contractable(pdf, 'client'), false);
  assert.equal(contractable({ ...pdf, name: 'acta.docx' }, 'owner'), false);
  assert.equal(contractable({ ...pdf, sealed: true }, 'owner'), false);
  assert.equal(contractable({ ...pdf, contract_state: 'borrador' }, 'owner'), false);
});

test('the Client consent and the signed notice keep their plain meaning', () => {
  assert.match(CLIENT_CONSENT_TEXT, /solo formaliza el contrato si el despacho firma antes/);
  assert.match(CLIENT_CONSENT_TEXT, /no podr\u00e1s retirarla/);
  assert.match(CLIENT_CONSENT_TEXT, /revocaci\u00f3n de tu certificado/);
  assert.equal(
    CLIENT_SIGNED_NOTICE,
    'Tu firma qued\u00f3 registrada; el contrato a\u00fan no est\u00e1 formalizado.',
  );
});

test('signing stays disabled until the PDF digest is checked and consent confirmed', () => {
  const ready = { pdfChecked: true, confirmed: true, prepared: {}, signature: 'x' };
  assert.equal(clientSignReady(ready), true);
  for (const key of Object.keys(ready))
    assert.equal(clientSignReady({ ...ready, [key]: key === 'pdfChecked' ? false : null }), false);
  assert.equal(clientSignReady({ ...ready, confirmed: 'yes' }), false);
});

test('each party sees only the actions of its position and state', () => {
  const now = inputs.created_at_seconds + 10;
  const detail = requestDetail();
  const client = { id: inputs.client_account_id, role: 'client' };
  const firm = { id: inputs.firm_account_id, role: 'litigator' };
  const requester = { id: inputs.requester_account_id, role: 'litigator' };
  assert.deepEqual(requestActions(detail, client, now), {
    position: 1,
    sign: true,
    reject: true,
    decline: false,
    cancel: false,
    partial: false,
    final: false,
    signedByMe: false,
  });
  assert.equal(requestActions(detail, firm, now).sign, false);
  assert.equal(requestActions(detail, requester, now).cancel, true);
  assert.equal(requestActions(detail, { id: 'other', role: 'litigator' }, now).cancel, false);
  assert.equal(requestActions(detail, { id: 'owner', role: 'owner' }, now).cancel, true);
  const signed = requestDetail({ status: 'awaiting_firm', clientSignedAt: '2026-11-02T01:00:00Z' });
  assert.equal(requestActions(signed, client, now).partial, true);
  assert.equal(requestActions(signed, client, now).reject, false);
  assert.equal(requestActions(signed, client, now).signedByMe, true);
  assert.equal(requestActions(signed, firm, now).sign, true);
  assert.equal(requestActions(signed, firm, now).decline, true);
  const expired = requestActions(signed, firm, inputs.expires_at_seconds + 1);
  assert.equal(expired.sign || expired.decline || expired.cancel, false);
  assert.equal(requestActions(requestDetail({ status: 'completed' }), client, now).final, true);
});

test('readiness blockers and declared names read in plain language', () => {
  const lines = blockerMessages(['client_not_ready', 'document_not_ready']);
  assert.equal(lines.length, 2);
  assert.match(lines[0], /Cliente/);
  assert.match(lines[1], /PDF/);
  assert.deepEqual(blockerMessages(undefined), []);
  assert.equal(
    declaredName({ summary: { subject: 'CN=Cliente Demo,O=Despacho,C=MX' } }),
    'Cliente Demo',
  );
});

test('an expired certificate sends the party back to its signing binding', () => {
  const binding = (notAfter) => ({ certificate: { summary: { not_after_unix: notAfter } } });
  assert.equal(certificateExpired(binding(100), 100), true);
  assert.equal(certificateExpired(binding(101), 100), false);
  assert.equal(certificateRenewalDue(binding(100 + 29 * 86400), 100), true);
  assert.equal(certificateRenewalDue(binding(100 + 31 * 86400), 100), false);
});

test('request form values follow the limits of the HTTP contract', () => {
  const draft = {
    clientEmail: ' cliente@example.test ',
    firmSignerId: inputs.firm_account_id,
    ttlDays: String(DEFAULT_TTL_DAYS),
    version: 3,
    ceremonyReference: 'HF-0001',
  };
  assert.deepEqual(contractRequestValues(draft), {
    clientEmail: 'cliente@example.test',
    firmSignerId: inputs.firm_account_id,
    ttlDays: 7,
    version: 3,
    ceremonyReference: 'HF-0001',
  });
  for (const changed of [
    { clientEmail: 'cliente' },
    { firmSignerId: '' },
    { ttlDays: 0 },
    { ttlDays: 31 },
    { ttlDays: 2.5 },
    { version: 0 },
    { ceremonyReference: '' },
    { ceremonyReference: 'x'.repeat(81) },
  ])
    assert.throws(() => contractRequestValues({ ...draft, ...changed }));
  assert.equal(contractTitle(' Contrato '), 'Contrato');
  assert.throws(() => contractTitle('x'.repeat(161)));
  assert.equal(contractReason('', false), null);
  assert.throws(() => contractReason('', true));
  assert.throws(() => contractReason('a\u0007b', true));
});
