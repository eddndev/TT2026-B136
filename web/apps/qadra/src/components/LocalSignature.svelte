<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { readable } from 'svelte/store';
  import { watchLocalVisibility } from '../lib/local-crypto-visibility.mjs';
  import { createLocalCrypto } from '../lib/local-crypto-client.mjs';
  export let contextKey, input, certificateFingerprint, purpose;
  export let certificateDerBase64 = '';
  export let disabled = false;
  export let requiresSession = true;
  const admission = getContext('crypto-admission') ?? readable(false);
  let visible = true;
  $: blocked = disabled || !visible || (requiresSession && !$admission);
  onMount(() =>
    watchLocalVisibility((value) => {
      visible = value;
    }),
  );
  export let onsignature,
    oninvalidate = () => {};
  const crypto = createLocalCrypto();
  let keyInput, certificateInput;
  let password = '',
    error = '',
    busy = false,
    signed = false;
  let generation = 0,
    alive = true,
    previousContext,
    previousDisabled;
  $: if (contextKey !== previousContext || blocked !== previousDisabled) {
    previousContext = contextKey;
    previousDisabled = blocked;
    clear();
  }
  function clear() {
    generation++;
    crypto.cancel();
    password = '';
    if (keyInput) keyInput.value = '';
    if (certificateInput) certificateInput.value = '';
    busy = signed = false;
    error = '';
  }
  function cancel() {
    clear();
    oninvalidate(contextKey);
  }
  function changed() {
    generation++;
    crypto.cancel();
    busy = signed = false;
    error = '';
    oninvalidate(contextKey);
  }
  async function read(file, limit) {
    if (!file || file.size < 1 || file.size > limit)
      throw new Error('Selecciona un archivo valido.');
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (bytes.length !== file.size || bytes.length > limit) throw new Error('El archivo cambio.');
    return bytes;
  }
  async function sign() {
    if (busy || blocked || !input || !password) return;
    const ticket = ++generation,
      context = contextKey;
    const active = () => alive && ticket === generation && context === contextKey && !blocked;
    busy = true;
    signed = false;
    error = '';
    oninvalidate(contextKey);
    try {
      if (new TextEncoder().encode(password).length > 1024)
        throw new Error('La contrasena supera el limite admitido.');
      const encryptedKeyPem = new TextDecoder('utf-8', { fatal: true }).decode(
        await read(keyInput?.files?.[0], 65536),
      );
      const certificate = certificateDerBase64
        ? Uint8Array.from(atob(certificateDerBase64), (c) => c.charCodeAt(0))
        : await read(certificateInput?.files?.[0], 16384);
      const bytes = input instanceof Blob ? new Uint8Array(await input.arrayBuffer()) : input;
      if (!active()) return;
      const pending = crypto.run('sign', {
        encryptedKeyPem,
        password,
        certificate,
        bytes,
        certificateFingerprint,
      });
      password = '';
      if (keyInput) keyInput.value = '';
      const signatureBase64 = await pending;
      if (!active()) return;
      if (typeof signatureBase64 !== 'string' || atob(signatureBase64).length !== 384)
        throw new Error('La firma local no tiene el formato esperado.');
      signed = onsignature({ signatureBase64, contextKey: context }) === true;
    } catch (failure) {
      if (active() && failure.name !== 'AbortError') error = failure.message;
    } finally {
      if (active()) {
        password = '';
        if (keyInput) keyInput.value = '';
        if (certificateInput) certificateInput.value = '';
        busy = false;
      }
    }
  }
  onDestroy(() => {
    alive = false;
    clear();
  });
</script>

<section class="stack local-signature" aria-label={'Firma local: ' + purpose} aria-busy={busy}>
  <h3>Firmar en este dispositivo</h3>
  <p>{purpose}. La clave y su contrasena se usan solamente en este navegador.</p>
  <small class="fingerprint">Certificado esperado: {certificateFingerprint}</small>
  {#if !certificateDerBase64}
    <label
      >Certificado para firma local
      <input
        type="file"
        accept=".pem,.der,.cer,.crt"
        bind:this={certificateInput}
        disabled={blocked || busy}
        onchange={changed}
      />
    </label>
  {/if}
  <label
    >Clave privada cifrada
    <input
      type="file"
      accept=".pem"
      bind:this={keyInput}
      disabled={blocked || busy}
      onchange={changed}
    />
  </label>
  <label
    >Contrasena de la clave
    <input
      type="password"
      autocomplete="off"
      bind:value={password}
      disabled={blocked || busy}
      oninput={changed}
    />
  </label>
  <div class="action-row">
    <button type="button" class="secondary" disabled={blocked || busy || !password} onclick={sign}
      >Firmar localmente</button
    >
    <button type="button" class="text-button" disabled={blocked} onclick={cancel}
      >Limpiar firma local</button
    >
  </div>
  {#if busy}<p role="status">Firmando en este dispositivo...</p>{/if}
  {#if signed}<p role="status">Firma local preparada. Revisa y confirma la operacion.</p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</section>

<style>
  .local-signature {
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 16px;
  }
  .fingerprint {
    overflow-wrap: anywhere;
  }
  input {
    max-width: 100%;
  }
</style>
