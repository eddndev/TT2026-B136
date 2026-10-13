import { ownerCertificateInvalid as invalid } from './owner-certificate-binary.mjs';
import {
  ownerObject,
  ownerUuid,
  ownerPreparation,
  ownerReceipt,
  ownerSubmission,
} from './owner-certificate-values.mjs';

function publicSubmission(registration) {
  return ownerSubmission({
    statement_base64: registration.statement_base64,
    certificate_der_base64: registration.certificate.der_base64,
    signature_base64: registration.signature_base64,
  });
}

export function registrationIntent(preparation, signatureBase64) {
  ownerPreparation(preparation);
  const data = ownerSubmission({
    statement_base64: preparation.statement_base64,
    certificate_der_base64: preparation.certificate.der_base64,
    signature_base64: signatureBase64,
  });
  return Object.freeze({
    ownerId: preparation.owner_id,
    bindingId: preparation.binding_id,
    operation: 'register',
    data,
  });
}

export function withdrawalIntent(receipt) {
  ownerReceipt(receipt, undefined, undefined, true);
  return Object.freeze({
    ownerId: receipt.owner_id,
    bindingId: receipt.binding_id,
    operation: 'withdraw',
    data: Object.freeze({ expected_revision: 1 }),
    registration: publicSubmission(receipt.registration),
  });
}

function commandEvidence(command) {
  const withdrawal = command?.operation === 'withdraw';
  ownerObject(
    command,
    withdrawal
      ? ['ownerId', 'bindingId', 'operation', 'data', 'registration']
      : ['ownerId', 'bindingId', 'operation', 'data'],
  );
  ownerUuid(command.ownerId);
  ownerUuid(command.bindingId);
  if (!withdrawal && command.operation !== 'register') invalid();
  if (withdrawal) {
    ownerObject(command.data, ['expected_revision']);
    if (command.data.expected_revision !== 1) invalid();
  }
  return ownerSubmission(withdrawal ? command.registration : command.data);
}

export function reconcileOwnerIntent(command, receiptOrNull) {
  try {
    const expected = commandEvidence(command);
    if (receiptOrNull === null) return 'absent';
    const receipt = ownerReceipt(receiptOrNull, command.ownerId, command.bindingId);
    const observed = publicSubmission(receipt.registration);
    if (Object.keys(expected).some((name) => expected[name] !== observed[name])) return 'conflict';
    return command.operation === 'withdraw' && receipt.withdrawal === null
      ? 'unconfirmed'
      : 'matched';
  } catch {
    return 'conflict';
  }
}

export function uncertainOwnerWrite(error) {
  return (
    !Number.isInteger(error?.status) ||
    error.status >= 500 ||
    error.status === 401 ||
    error.status === 403
  );
}
