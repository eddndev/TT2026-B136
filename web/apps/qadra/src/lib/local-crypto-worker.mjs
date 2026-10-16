import init, {
  generate_key,
  sign_bytes,
  inspect_certificate,
  check_key_backup,
} from '../generated/browser-crypto/browser_crypto.js';
import wasmUrl from '../generated/browser-crypto/browser_crypto_bg.wasm?url';

function certificateDer(input) {
  if (!(input instanceof Uint8Array) || !input.length || input.length > 16384)
    throw new Error('certificate');
  if (input[0] === 0x30) return input;
  const text = new TextDecoder('utf-8', { fatal: true }).decode(input);
  const match =
    /^\s*-----BEGIN CERTIFICATE-----\s*([A-Za-z0-9+/=\r\n]+)\s*-----END CERTIFICATE-----\s*$/.exec(
      text,
    );
  if (!match) throw new Error('certificate');
  return Uint8Array.from(atob(match[1].replace(/\s/g, '')), (c) => c.charCodeAt(0));
}
function failureCode(error) {
  const code = typeof error === 'string' ? error : error?.message;
  if (code === 'unavailable') return 'unavailable';
  if (/entropy|random/.test(code ?? '')) return 'entropy';
  if (/certificate|mismatch/.test(code ?? '')) return 'certificate';
  if (/request|csr/.test(code ?? '')) return 'request';
  if (/key|password|decrypt|pbes|kdf/.test(code ?? '')) return 'key';
  return 'input';
}
self.onmessage = async ({ data: { type, payload } }) => {
  try {
    if (!self.crypto?.getRandomValues) throw new Error('entropy');
    try {
      await init({ module_or_path: wasmUrl });
    } catch {
      throw new Error('unavailable');
    }
    let result;
    if (type === 'generate') {
      const value = JSON.parse(
        generate_key(payload.password, payload.commonName, payload.organization, payload.country),
      );
      result = { encryptedKeyPem: value.encrypted_key_pem, csrPem: value.csr_pem };
    } else if (type === 'check_key') {
      result = check_key_backup(payload.encryptedKeyPem, payload.password, payload.csrPem);
    } else if (type === 'sign') {
      const der = certificateDer(payload.certificate);
      const info = JSON.parse(inspect_certificate(der));
      if (
        !/^[0-9a-f]{64}$/.test(payload.certificateFingerprint) ||
        info.certificate_sha256 !== payload.certificateFingerprint
      )
        throw new Error('certificate');
      const signature = sign_bytes(payload.encryptedKeyPem, payload.password, der, payload.bytes);
      result = btoa(String.fromCharCode(...signature));
      signature.fill(0);
    } else throw new Error('input');
    self.postMessage({ ok: true, result });
  } catch (error) {
    self.postMessage({ ok: false, error: failureCode(error) });
  } finally {
    payload.password = '';
    payload.encryptedKeyPem = '';
    self.close();
  }
};
