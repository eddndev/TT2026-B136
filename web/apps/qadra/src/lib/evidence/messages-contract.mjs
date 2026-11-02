// Spanish messages for the findings about the request and the party
// statements of bilateral contract signing. The codes are the stable codes of
// crates/evidence-verify. Rationale:
// docs/adr/0086-bilateral-contract-signing.md.

export const contractMessages = {
  'contract.request_schema': 'La solicitud de firma del contrato no tiene el formato esperado.',
  'statement.position':
    'Una declaraci\u00f3n firmada del contrato no indica un orden de firma v\u00e1lido.',
};

export const contractFallbacks = {
  contract: 'La firma bilateral del contrato tiene un problema.',
};
