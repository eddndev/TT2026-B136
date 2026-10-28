// One evidence validation inside the Web Worker, kept free of browser globals
// so that it can be tested in Node. The worker loads the module, runs one
// synchronous validation and posts coarse stages and the result, tagged with
// the operation identifier it received. Errors are reduced to fixed codes so
// that no engine detail leaves the worker. Rationale:
// docs/adr/0085-in-browser-evidence-validation.md.

function report(text, sourceDigest) {
  const value = JSON.parse(text);
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('report');
  if (!value.engine || typeof value.engine !== 'object') throw new Error('report');
  value.engine = { ...value.engine, source_digest: sourceDigest };
  return value;
}

/**
 * Validate `data.zip` (an ArrayBuffer) under `data.policy`. `load` prepares
 * the module, `validate(bytes, policyJson)` is the module export and `post`
 * sends one message to the client.
 */
export async function handleValidation(data, { load, validate, sourceDigest, post }) {
  const op = data?.op;
  const fail = (error) => post({ op, ok: false, error });
  if (!(data?.zip instanceof ArrayBuffer) || !data.policy || typeof data.policy !== 'object') {
    fail('engine_failed');
    return;
  }
  post({ op, stage: 'module' });
  try {
    await load();
  } catch {
    fail('module_unavailable');
    return;
  }
  post({ op, stage: 'validate' });
  let text;
  try {
    text = validate(new Uint8Array(data.zip), JSON.stringify(data.policy));
  } catch (error) {
    fail(
      typeof error === 'string' && error.startsWith('policy.') ? 'policy_invalid' : 'engine_failed',
    );
    return;
  }
  let value;
  try {
    value = report(text, sourceDigest);
  } catch {
    fail('engine_failed');
    return;
  }
  post({ op, ok: true, report: value });
}
