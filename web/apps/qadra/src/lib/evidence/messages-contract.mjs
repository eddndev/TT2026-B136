// Spanish messages for the findings about the request, the party statements,
// the manifest and the receipts of bilateral contract signing. The codes are
// the stable codes of crates/evidence-verify. Rationale:
// docs/adr/0086-bilateral-contract-signing.md.

export const contractMessages = {
  'contract.request_schema': 'La solicitud de firma del contrato no tiene el formato esperado.',
  'statement.position':
    'Una declaraci\u00f3n firmada del contrato no indica un orden de firma v\u00e1lido.',
  'manifest.contract_schema':
    'El manifiesto del contrato no tiene la forma exacta que Qadra escribe.',
  'receipt.contract_schema': 'El recibo del contrato no tiene la forma exacta que Qadra escribe.',
  'receipt.contract_partial_schema':
    'El acuse parcial del contrato no tiene la forma exacta que Qadra escribe.',
};

export const contractFallbacks = {
  contract: 'La firma bilateral del contrato tiene un problema.',
};
