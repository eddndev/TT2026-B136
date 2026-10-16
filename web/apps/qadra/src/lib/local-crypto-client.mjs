const messages = {
  key: 'No se pudo abrir la clave cifrada. Comprueba el archivo y su contrasena.',
  certificate: 'El certificado no es valido o no corresponde a la clave.',
  request: 'La solicitud de certificado no corresponde a esta clave.',
  input: 'Los datos de la operacion local no son validos.',
  entropy: 'El navegador no ofrece una fuente segura para generar la clave.',
};
const unavailable = 'No se pudo iniciar la criptografia local en este navegador.';
export function createLocalCrypto(
  factory = () =>
    new Worker(new URL('./local-crypto-worker.mjs', import.meta.url), { type: 'module' }),
) {
  let current = null;
  function cancel() {
    current?.finish(new DOMException('Operacion local cancelada.', 'AbortError'));
  }
  function run(type, payload) {
    cancel();
    return new Promise((resolve, reject) => {
      let worker;
      try {
        worker = factory();
      } catch {
        reject(new Error(unavailable));
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
        if (data?.ok === true) operation.finish(null, data.result);
        else operation.finish(new Error(messages[data?.error] ?? unavailable));
      };
      worker.onerror = worker.onmessageerror = () => operation.finish(new Error(unavailable));
      try {
        worker.postMessage({ type, payload });
      } catch {
        operation.finish(new Error(unavailable));
      }
    });
  }
  return { run, cancel };
}
