// The dedicated Web Worker of the evidence validator. It runs one validation
// of a local ZIP and closes; the file never leaves the browser. Rationale:
// docs/adr/0085-in-browser-evidence-validation.md.
import init, { validate } from '../generated/evidence-verify/evidence_verify_wasm.js';
import wasmUrl from '../generated/evidence-verify/evidence_verify_wasm_bg.wasm?url';
import manifest from '../generated/evidence-verify/manifest.json';
import { handleValidation } from './evidence-verify-run.mjs';

self.onmessage = async ({ data }) => {
  try {
    await handleValidation(data, {
      load: () => init({ module_or_path: wasmUrl }),
      validate,
      sourceDigest: manifest.source,
      post: (message) => self.postMessage(message),
    });
  } finally {
    self.close();
  }
};
