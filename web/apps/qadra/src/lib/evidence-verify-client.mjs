// Runs evidence validations in a dedicated Web Worker, one worker per
// operation. A newer run or cancel() ends the previous one; replies of an
// ended operation, or carrying another operation identifier, are dropped.
// Rationale: docs/adr/0085-in-browser-evidence-validation.md.
const messages = {
  module_unavailable: 'No se pudo iniciar el validador de evidencia en este navegador.',
  policy_invalid:
    'La lista de ra\u00edces de confianza de esta versi\u00f3n de Qadra no es v\u00e1lida.',
  engine_failed: 'El validador no pudo terminar la revisi\u00f3n del archivo.',
};
const STAGES = ['module', 'validate'];

function failure(code) {
  const known = Object.hasOwn(messages, code) ? code : 'engine_failed';
  const error = new Error(messages[known]);
  error.code = known;
  return error;
}

export function createEvidenceVerifier(
  factory = () =>
    new Worker(new URL('./evidence-verify-worker.mjs', import.meta.url), { type: 'module' }),
) {
  let current = null;
  let counter = 0;
  function cancel() {
    current?.finish(new DOMException('Validaci\u00f3n cancelada.', 'AbortError'));
  }
  function run({ context, zip, policy }, onStage = () => {}) {
    cancel();
    if (!(zip instanceof ArrayBuffer)) return Promise.reject(new TypeError('zip'));
    counter += 1;
    const op = counter;
    return new Promise((resolve, reject) => {
      let worker;
      try {
        worker = factory();
      } catch {
        reject(failure('module_unavailable'));
        return;
      }
      const operation = {
        finish(error, result) {
          if (current !== operation) return;
          current = null;
          worker.onmessage = worker.onerror = worker.onmessageerror = null;
          worker.terminate();
          if (error) reject(error);
          else resolve(result);
        },
      };
      current = operation;
      worker.onmessage = ({ data }) => {
        if (current !== operation || data?.op !== op) return;
        if ('stage' in data) {
          if (STAGES.includes(data.stage)) onStage(data.stage);
        } else if (data.ok === true) operation.finish(null, { context, report: data.report });
        else operation.finish(failure(data.error));
      };
      worker.onerror = () => operation.finish(failure('module_unavailable'));
      worker.onmessageerror = () => operation.finish(failure('engine_failed'));
      try {
        worker.postMessage({ op, zip, policy }, [zip]);
      } catch {
        operation.finish(failure('engine_failed'));
      }
    });
  }
  return { run, cancel };
}
