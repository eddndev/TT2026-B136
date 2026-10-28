// Replaces the evidence validation Web Worker in browser tests. The bytes of
// the chosen file are JSON naming the scenario: {"overall", "delay"}. The
// stub follows the worker protocol of src/lib/evidence-verify-worker.mjs and
// never reads the network.
const finding = (outcome, code, detail = '') => ({ outcome, code, detail });
const component = (kind, outcome, findings) => ({ kind, outcome, findings });

function report(scenario, policy) {
  const label = policy.pinned[0].label;
  const components = [
    component('package', 'checked', [finding('checked', 'zip.admitted')]),
    component('document-token', 'checked', [
      finding('checked', 'token.verified'),
      finding('checked', 'token.anchor_pinned', `${label} for time-stamping`),
    ]),
    component('document-signature', 'checked', [finding('checked', 'signature.valid')]),
    component('binding-consent', 'not-applicable', []),
    component('certificates', 'checked', [
      finding('checked', 'cert.profile', 'certificado.pem: Signer'),
      finding('checked', 'trust.root_pinned', `ca.pem: ${label} for server-seal`),
    ]),
    component('revocation', 'checked', [finding('checked', 'crl.not_revoked')]),
    component('manifest', 'not-applicable', []),
    component('context', 'checked', [finding('checked', 'context.token_time')]),
  ];
  if (scenario.overall === 'failed')
    components[2] = component('document-signature', 'failed', [
      finding('failed', 'signature.invalid'),
    ]);
  if (scenario.overall === 'indeterminate')
    components[5] = component('revocation', 'indeterminate', [
      finding('indeterminate', 'crl.not_covering', 'crl.pem'),
    ]);
  const unsupported = scenario.overall === 'not-supported';
  return {
    engine: { name: 'evidence-verify', version: 'stub', source_digest: 'stub-digest' },
    profile: unsupported ? null : 'server-seal-v1',
    overall: scenario.overall,
    components: unsupported
      ? [component('package', 'not-supported', [finding('not-supported', 'zip.method', 'a')])]
      : components,
    instants: [
      { label: 'document token genTime', unix: 1793145600, source: 'token' },
      { label: 'current status', unix: policy.current_instant, source: 'caller' },
    ],
    digests: unsupported ? [] : [{ name: 'contrato.pdf', sha256: 'a'.repeat(64) }],
    trust:
      policy.served === null
        ? [{ code: 'trust.route_unavailable', purpose: null, fingerprint: '', detail: '' }]
        : [],
    current_status: [
      finding('checked', 'current.instant', String(policy.current_instant)),
      finding('checked', 'current.within_validity', 'server seal signer: valid now'),
    ],
  };
}

self.onmessage = async ({ data }) => {
  const { op } = data;
  const scenario = JSON.parse(new TextDecoder().decode(new Uint8Array(data.zip)));
  self.postMessage({ op, stage: 'module' });
  self.postMessage({ op, stage: 'validate' });
  await new Promise((resolve) => setTimeout(resolve, scenario.delay || 0));
  if (scenario.overall === 'error') self.postMessage({ op, ok: false, error: 'engine_failed' });
  else self.postMessage({ op, ok: true, report: report(scenario, data.policy) });
};
