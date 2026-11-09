<script>
  import Button from './Button.svelte';
  import { getContext, onMount, onDestroy } from 'svelte';
  import { readable } from 'svelte/store';
  import { watchLocalVisibility } from '../lib/local-crypto-visibility.mjs';
  import { download } from '../lib/documents.mjs';
  import { keyRequestValues, generatedKeyRequest } from '../lib/key-request-values.mjs';
  import { createLocalCrypto } from '../lib/local-crypto-client.mjs';
  import KeyFingerprintSheet from './KeyFingerprintSheet.svelte';

  export let contextKey;
  export let purpose;
  export let disabled = false;
  // A Client certificate carries the organization of the issuing CA by
  // policy, so the field is fixed. Rationale:
  // docs/adr/0086-bilateral-contract-signing.md.
  export let organizationLocked = false;
  export let accountEmail = '';
  const admission = getContext('crypto-admission') ?? readable(false);
  let visible = true,
    previousAllowed;
  onMount(() =>
    watchLocalVisibility((value) => {
      visible = value;
    }),
  );
  $: if (allowed !== previousAllowed) {
    previousAllowed = allowed;
    clear();
  }

  let commonName = '',
    organization = 'Qadra',
    country = 'MX',
    password = '',
    passwordConfirmation = '',
    checkPassword = '',
    error = '';
  let generated = null,
    selectedFile = null,
    fileInput,
    downloaded = false,
    verified = false,
    busy = false,
    generation = 0,
    alive = true,
    cryptoClient = null;
  let previousContext, previousPurpose, previousDisabled;

  $: allowed =
    !!contextKey &&
    !disabled &&
    $admission &&
    visible &&
    ['partner', 'document-signing'].includes(purpose);
  $: if (
    previousContext !== contextKey ||
    previousPurpose !== purpose ||
    previousDisabled !== disabled
  ) {
    previousContext = contextKey;
    previousPurpose = purpose;
    previousDisabled = disabled;
    clear();
  }

  function client() {
    cryptoClient ??= createLocalCrypto();
    return cryptoClient;
  }
  function active(ticket) {
    return alive && allowed && ticket === generation;
  }
  function clear() {
    generation++;
    cryptoClient?.cancel();
    commonName = '';
    organization = 'Qadra';
    country = 'MX';
    password = passwordConfirmation = checkPassword = error = '';
    generated = selectedFile = null;
    downloaded = verified = busy = false;
    if (fileInput) fileInput.value = '';
  }
  async function generate(event) {
    event.preventDefault();
    if (!allowed || busy || generated) return;
    let request;
    try {
      request = keyRequestValues({
        commonName,
        organization,
        country,
        password,
        passwordConfirmation,
      });
    } catch (failure) {
      error = failure.message;
      return;
    }
    const ticket = ++generation;
    busy = true;
    error = '';
    try {
      const pending = client().run('generate', request);
      request = null;
      password = passwordConfirmation = '';
      const result = await pending;
      if (!active(ticket)) return;
      generated = generatedKeyRequest(result);
    } catch (failure) {
      if (active(ticket)) error = failure.message;
    } finally {
      request = null;
      if (ticket === generation) {
        password = passwordConfirmation = '';
        busy = false;
      }
    }
  }
  function saveKey() {
    if (!allowed || busy || !generated) return;
    error = '';
    try {
      download(
        new Blob([generated.encryptedKeyPem], { type: 'application/octet-stream' }),
        `qadra-${purpose}-key.encrypted.pem`,
      );
      downloaded = true;
    } catch {
      error = 'No se pudo descargar la clave cifrada. Vuelve a intentarlo.';
    }
  }
  function selectFile(event) {
    if (!allowed || busy) return;
    selectedFile = event.currentTarget.files?.[0] ?? null;
    verified = false;
    checkPassword = error = '';
  }
  function changeCheckPassword(event) {
    checkPassword = event.currentTarget.value;
    verified = false;
  }
  async function checkSavedKey(event) {
    event.preventDefault();
    if (!allowed || busy || !generated || !downloaded) return;
    verified = false;
    const file = selectedFile;
    if (!file || !Number.isSafeInteger(file.size) || file.size < 1 || file.size > 65536) {
      error = 'Selecciona la clave cifrada guardada, de hasta 64 KiB.';
      return;
    }
    if (!checkPassword || new TextEncoder().encode(checkPassword).byteLength > 1024) {
      error = 'Introduce la contrase\u00f1a de la clave guardada, de hasta 1024 bytes.';
      return;
    }
    const ticket = ++generation;
    busy = true;
    error = '';
    let encryptedKeyPem = '';
    try {
      const bytes = await file.arrayBuffer();
      if (!active(ticket)) return;
      if (!(bytes instanceof ArrayBuffer) || !bytes.byteLength || bytes.byteLength > 65536)
        throw new Error('Invalid encrypted key size');
      encryptedKeyPem = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
      const pending = client().run('check_key', {
        encryptedKeyPem,
        password: checkPassword,
        csrPem: generated.csrPem,
      });
      checkPassword = encryptedKeyPem = '';
      const result = await pending;
      if (!active(ticket)) return;
      if (result !== true) throw new Error('Saved key does not match');
      verified = true;
    } catch {
      if (active(ticket))
        error =
          'No se pudo comprobar la copia. Revisa el archivo y su contrase\u00f1a; la clave debe corresponder a esta solicitud.';
    } finally {
      encryptedKeyPem = '';
      if (ticket === generation) {
        checkPassword = '';
        busy = false;
      }
    }
  }
  function saveRequest() {
    if (!allowed || busy || !verified || !generated) return;
    error = '';
    try {
      download(
        new Blob([generated.csrPem], { type: 'application/pkcs10' }),
        `qadra-${purpose}-request.csr.pem`,
      );
    } catch {
      error = 'No se pudo descargar la solicitud. Vuelve a intentarlo.';
    }
  }
  onDestroy(() => {
    alive = false;
    clear();
  });
</script>

<section class="card stack key-request" aria-label="Generar clave y solicitud" aria-busy={busy}>
  <div class="section-heading">
    <h2>Generar clave y solicitud</h2>
    <span class="badge info">En este navegador</span>
  </div>
  <p>
    Crea una clave protegida por contrase&#241;a y una solicitud p&#250;blica (CSR) para
    {purpose === 'partner' ? 'acceso de administrador' : 'firma documental'}. La clave y su
    contrase&#241;a permanecen en tu equipo. El responsable de la CA revisa y emite el certificado.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !generated}
    <form class="stack" onsubmit={generate}>
      <fieldset class="key-request-fields" disabled={!allowed || busy}>
        <legend>Datos de la solicitud</legend>
        <label
          >Nombre com&#250;n (CN)<input
            required
            maxlength="128"
            bind:value={commonName}
            autocomplete="off"
          /></label
        >
        <label
          >Organizaci&#243;n (O)<input
            required
            maxlength="128"
            bind:value={organization}
            readonly={organizationLocked}
            autocomplete="off"
          /></label
        >
        {#if organizationLocked}<p class="hint">
            La organizaci&#243;n es la de la CA emisora por pol&#237;tica; no indica que pertenezcas
            al despacho.
          </p>{/if}
        <label
          >Pa&#237;s (C)<input
            required
            maxlength="2"
            pattern={'[A-Z]{2}'}
            bind:value={country}
            autocomplete="off"
          /></label
        >
      </fieldset>
      <p class="hint">
        El nombre es declarado; no acredita identidad. Organizaci&#243;n y pa&#237;s deben coincidir
        con la CA emisora.
      </p>
      <label
        >Contrase&#241;a de la nueva clave
        <input
          type="password"
          required
          maxlength="1024"
          autocomplete="new-password"
          bind:value={password}
          disabled={!allowed || busy}
        />
      </label>
      <label
        >Confirmar contrase&#241;a
        <input
          type="password"
          required
          maxlength="1024"
          autocomplete="new-password"
          bind:value={passwordConfirmation}
          disabled={!allowed || busy}
        />
      </label>
      <p class="hint">
        Usa al menos 12 caracteres. Guarda la clave y su contrase&#241;a: Qadra no puede
        recuperarlas.
      </p>
      <Button variant="primary" type="submit" disabled={!allowed || busy}
        >{busy ? 'Generando...' : 'Generar clave y solicitud'}</Button
      >
    </form>
  {:else}
    <p>
      Solicitud preparada para <strong>{commonName}</strong>. Guarda primero tu clave cifrada y
      comprueba la copia.
    </p>
    <KeyFingerprintSheet
      spkiSha256={generated.spkiSha256}
      {commonName}
      {organization}
      {country}
      email={accountEmail}
    />
    <Button variant="primary" type="button" disabled={!allowed || busy} onclick={saveKey}
      >Descargar clave cifrada</Button
    >
    {#if downloaded}
      <form class="stack" onsubmit={checkSavedKey}>
        <label
          >Archivo de clave cifrada guardado
          <input
            type="file"
            accept=".pem"
            bind:this={fileInput}
            disabled={!allowed || busy}
            onchange={selectFile}
          />
        </label>
        <label
          >Contrase&#241;a de la clave guardada
          <input
            type="password"
            maxlength="1024"
            autocomplete="off"
            value={checkPassword}
            disabled={!allowed || busy}
            oninput={changeCheckPassword}
          />
        </label>
        <Button
          variant="secondary"
          type="submit"
          disabled={!allowed || busy || !selectedFile || !checkPassword}
        >
          {busy ? 'Comprobando...' : 'Comprobar copia guardada'}
        </Button>
      </form>
    {/if}
    {#if verified}
      <p class="notice success" role="status">
        La copia cifrada se puede abrir y corresponde a esta solicitud.
      </p>
      <Button variant="primary" type="button" disabled={!allowed || busy} onclick={saveRequest}
        >Descargar solicitud CSR</Button
      >
      <p class="hint">
        Comparte s&#243;lo el CSR con el responsable de la CA. Emitir el certificado no lo vincula a
        tu cuenta.
      </p>
    {/if}
  {/if}
  <Button variant="secondary" type="button" onclick={clear}>Limpiar datos locales</Button>
</section>

<style>
  .key-request {
    margin-top: 24px;
  }
  .key-request-fields {
    display: grid;
    gap: 16px;
    border: 0;
    padding: 0;
    margin: 0;
  }
  .key-request-fields legend {
    margin-bottom: 12px;
    font-weight: 500;
  }
  .key-request input {
    width: 100%;
  }
  .key-request button {
    align-self: start;
  }
</style>
