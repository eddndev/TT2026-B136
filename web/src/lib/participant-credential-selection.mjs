import {
  readPublicCertificate,
  readDetachedSignature,
  declarationDownloads,
} from './participant-credential-files.mjs';

export function credentialSelection() {
  let scope, basis;
  let generation = 0;
  let preparing = null;
  let alive = true;
  let certificate = null,
    signature = null,
    prepared = null,
    busy = false,
    error = '';
  const listeners = new Set();
  const available = () => alive && scope !== null && scope !== undefined;
  function snapshot() {
    return Object.freeze({
      available: available(),
      certificate,
      signature,
      prepared,
      busy,
      error,
      ready: !!(certificate && prepared && signature && !busy),
    });
  }
  function notify() {
    const value = snapshot();
    for (const listener of listeners) listener(value);
  }
  function invalidate(clearCertificate = false) {
    generation++;
    preparing = null;
    if (clearCertificate) certificate = null;
    prepared = null;
    signature = null;
    busy = false;
    error = '';
  }
  const current = (ticket) => alive && ticket === generation;
  async function select(file, kind) {
    if (!available() || (kind === 'signature' && preparing !== null)) return false;
    if (kind === 'certificate') invalidate(true);
    else {
      generation++;
      signature = null;
      error = '';
      if (file && !prepared) {
        error = 'Prepara la declaraci\u00f3n antes de seleccionar su firma.';
        notify();
        return false;
      }
    }
    busy = !!file;
    const ticket = generation;
    notify();
    if (!file) return true;
    try {
      const blob = await (kind === 'certificate'
        ? readPublicCertificate(file)
        : readDetachedSignature(file));
      if (!current(ticket)) return false;
      const selected = Object.freeze({ name: file.name || 'Archivo seleccionado', blob });
      if (kind === 'certificate') certificate = selected;
      else signature = selected;
      return true;
    } catch (failure) {
      if (current(ticket)) error = failure.message;
      return false;
    } finally {
      if (current(ticket)) {
        busy = false;
        notify();
      }
    }
  }
  return {
    snapshot,
    subscribe(listener) {
      listener(snapshot());
      if (alive) listeners.add(listener);
      return () => listeners.delete(listener);
    },
    updateContext(nextScope, nextBasis) {
      if (!alive || (Object.is(scope, nextScope) && Object.is(basis, nextBasis))) return;
      invalidate(!Object.is(scope, nextScope));
      scope = nextScope;
      basis = nextBasis;
      notify();
    },
    selectCertificate: (file) => select(file, 'certificate'),
    selectSignature: (file) => select(file, 'signature'),
    beginPreparation() {
      if (!available() || busy || !certificate)
        throw new Error(
          'Selecciona un certificado p\u00fablico antes de preparar la declaraci\u00f3n.',
        );
      invalidate();
      busy = true;
      preparing = generation;
      notify();
      return generation;
    },
    acceptPreparation(ticket, statementBytes, receiptBytes) {
      if (!current(ticket) || preparing !== ticket || !certificate) return false;
      preparing = null;
      try {
        prepared = declarationDownloads(statementBytes, receiptBytes);
        error = '';
        return true;
      } catch (failure) {
        error = failure.message;
        return false;
      } finally {
        busy = false;
        notify();
      }
    },
    failPreparation(ticket, message) {
      if (!current(ticket) || preparing !== ticket) return false;
      preparing = null;
      busy = false;
      error = message;
      notify();
      return true;
    },
    dispose() {
      if (!alive) return;
      invalidate(true);
      alive = false;
      notify();
      listeners.clear();
    },
  };
}
