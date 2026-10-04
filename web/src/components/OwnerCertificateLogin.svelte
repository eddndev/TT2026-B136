<script>
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { download } from '../lib/documents.mjs';
  import {
    readOwnerLoginReceipt,
    ownerLoginStatementDownload,
    ownerLoginDeadline,
  } from '../lib/owner-login-values.mjs';
  import { readDetachedSignature } from '../lib/participant-credential-files.mjs';
  import { bytesBase64 } from '../lib/typed-participant-preparation.mjs';
  export let api, onchallenge, onclose;
  export let initialSelection = null,
    initialError = '';
  let selected = initialSelection;
  let error = initialError;
  let available = null,
    busy = false,
    alive = true,
    generation = 0;
  let prepared = null,
    deadline = 0,
    signatureBase64 = '',
    retry = !!initialError;
  const unavailable = 'El acceso con certificado no est\u00e1 disponible en este servidor.';
  const active = (request) => alive && request === generation;
  function clearProof() {
    prepared = null;
    signatureBase64 = '';
    deadline = 0;
  }
  function expired() {
    if (!prepared || performance.now() < deadline) return false;
    generation++;
    api.cancelAuthentication();
    clearProof();
    busy = false;
    retry = true;
    error = 'El intento de acceso venci\u00f3. Prepara otro intento y firma los nuevos bytes.';
    return true;
  }
  function failed(failure) {
    api.cancelAuthentication();
    clearProof();
    retry = true;
    if (failure.status === 404) {
      available = false;
      selected = null;
      error = unavailable;
    } else error = 'No se pudo confirmar el acceso. Prepara otro intento y una nueva firma.';
  }
  async function loadAvailability() {
    const request = ++generation;
    try {
      const enabled = await api.certificateLoginAvailable();
      if (!active(request)) return;
      available = enabled;
      if (!enabled) error = unavailable;
    } catch {
      if (active(request)) {
        available = false;
        error = unavailable;
      }
    }
  }
  async function receipt(event) {
    const file = event.currentTarget.files[0];
    event.currentTarget.value = '';
    if (!available || busy) return;
    const request = ++generation;
    api.cancelAuthentication();
    selected = null;
    clearProof();
    retry = false;
    error = '';
    if (!file) return;
    busy = true;
    try {
      const value = await readOwnerLoginReceipt(file);
      if (active(request)) selected = value;
    } catch {
      if (active(request))
        error =
          'Selecciona solamente el recibo p\u00fablico JSON de un v\u00ednculo sin retirar, sin claves privadas.';
    } finally {
      if (active(request)) busy = false;
    }
  }
  async function prepare() {
    if (!available || busy || !selected) return;
    const request = ++generation,
      selection = selected;
    clearProof();
    busy = true;
    error = '';
    const startedAt = performance.now();
    try {
      const value = await api.startCertificateLogin(selection);
      if (!active(request)) return;
      deadline = ownerLoginDeadline(value, { startedAt, receivedAt: performance.now() });
      prepared = value;
      retry = false;
    } catch (failure) {
      if (active(request)) failed(failure);
    } finally {
      if (active(request)) busy = false;
    }
  }
  async function signature(event) {
    const file = event.currentTarget.files[0];
    event.currentTarget.value = '';
    if (!available || busy || !prepared || expired()) return;
    const request = ++generation;
    signatureBase64 = '';
    error = '';
    if (!file) return;
    busy = true;
    try {
      const blob = await readDetachedSignature(file);
      if (!active(request) || expired()) return;
      const encoded = await bytesBase64(blob);
      if (active(request) && !expired()) signatureBase64 = encoded;
    } catch {
      if (active(request))
        error = 'Selecciona una firma binaria separada de exactamente 384 bytes.';
    } finally {
      if (active(request)) busy = false;
    }
  }
  function statement() {
    if (!alive || busy || !prepared || expired()) return;
    download(ownerLoginStatementDownload(prepared, selected), 'acceso-certificado.bin');
  }
  async function prove() {
    if (!available || busy || !prepared || !signatureBase64 || expired()) return;
    const request = ++generation,
      selection = selected;
    const token = prepared.challenge_token,
      signed = signatureBase64;
    clearProof();
    busy = true;
    error = '';
    const startedAt = performance.now();
    try {
      const value = await api.proveCertificateLogin(token, signed);
      if (!active(request)) return;
      const until = ownerLoginDeadline(value, { startedAt, receivedAt: performance.now() });
      onchallenge(value, selection, until);
    } catch (failure) {
      if (active(request)) failed(failure);
    } finally {
      if (active(request)) busy = false;
    }
  }
  function close() {
    generation++;
    clearProof();
    selected = null;
    onclose();
  }
  onMount(() => {
    loadAvailability();
    const timer = setInterval(expired, 500);
    return () => clearInterval(timer);
  });
  onDestroy(() => {
    alive = false;
    generation++;
    clearProof();
    selected = null;
  });
</script>

<section
  class="stack certificate-login"
  aria-label="Acceso con certificado"
  aria-busy={busy || available === null}
>
  <span class="auth-symbol"><Icon name="shield" size={24} /></span>
  <span class="eyebrow">PASO 01 / FIRMA EXTERNA</span>
  <h2>Ingresa con tu certificado.</h2>
  <p>Usa el recibo de tu v&iacute;nculo. Tu clave privada permanece fuera de Qadra.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if available === null}
    <p role="status">Consultando disponibilidad...</p>
  {:else if available}
    <label
      >Recibo p&uacute;blico del v&iacute;nculo
      <input type="file" accept=".json,application/json" disabled={busy} onchange={receipt} />
    </label>
    <small>Selecciona el JSON descargado de Mi certificado. No selecciones una clave privada.</small
    >
    {#if selected}
      <dl class="selected-receipt">
        <dt>Nombre declarado en el recibo</dt>
        <dd>{selected.subject}</dd>
        <dt>Cuenta Owner</dt>
        <dd>{selected.ownerId}</dd>
        <dt>V&iacute;nculo</dt>
        <dd>{selected.bindingId}</dd>
        <dt>Huella del certificado</dt>
        <dd>{selected.leafFingerprint}</dd>
      </dl>
      <small
        >El recibo es hist&oacute;rico. El servidor comprueba la cuenta, el v&iacute;nculo y la
        confianza actual en cada intento.</small
      >
    {/if}
    {#if prepared}
      <p>
        Descarga estos 182 bytes y f&iacute;rmalos fuera de Qadra con la clave del certificado
        vinculado. La firma s&oacute;lo sirve para este intento de acceso.
      </p>
      <button class="secondary" type="button" disabled={busy} onclick={statement}
        >Descargar bytes de acceso</button
      >
      <details>
        <summary>Firmar externamente con OpenSSL</summary>
        <p>Ejecuta este ejemplo en tu propio equipo. Conserva la clave fuera del navegador.</p>
        <code
          >openssl dgst -sha256 -sign clave-privada.pem -sigopt rsa_padding_mode:pkcs1 -out
          firma-acceso.sig acceso-certificado.bin</code
        >
      </details>
      <label
        >Firma separada de acceso
        <input
          type="file"
          accept=".sig,application/octet-stream"
          disabled={busy}
          onchange={signature}
        />
      </label>
      <small
        >{signatureBase64
          ? 'Firma de 384 bytes seleccionada.'
          : 'Selecciona la firma binaria externa de exactamente 384 bytes.'}</small
      >
      <button class="primary" type="button" disabled={busy || !signatureBase64} onclick={prove}
        >Comprobar firma</button
      >
    {:else if busy}
      <p role="status">Procesando el intento...</p>
    {:else}
      <button class="primary" type="button" disabled={!selected} onclick={prepare}
        >{retry ? 'Preparar otro intento' : 'Preparar acceso'}</button
      >
    {/if}
    <small
      >La firma no abre una sesi&oacute;n. Despu&eacute;s debes confirmar el segundo factor.</small
    >
  {/if}
  <button class="text-button" type="button" onclick={close}>Volver a contrase&ntilde;a</button>
</section>

<style>
  .certificate-login {
    min-width: 0;
  }
  .selected-receipt {
    margin: 0;
  }
  .selected-receipt dt {
    font-weight: 600;
  }
  .selected-receipt dd {
    margin: 0 0 8px;
    overflow-wrap: anywhere;
  }
  details code {
    display: block;
    white-space: normal;
    overflow-wrap: anywhere;
  }
</style>
