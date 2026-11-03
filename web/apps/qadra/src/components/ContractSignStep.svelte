<script>
  // The signing step of a contract party: the server prepares the 303-byte
  // CTRTSIG1 statement for this operation, the party signs those exact
  // bytes with its key in this browser (or outside Qadra) and sends only the
  // signature. An uncertain answer is reconciled through the receipt of the
  // same operation; nothing is resent automatically.
  // Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { onDestroy } from 'svelte';
  import { ownerBase64 } from '../lib/owner-certificate-binary.mjs';
  import { download } from '../lib/documents.mjs';
  import LocalSignature from './LocalSignature.svelte';
  import SigningExternalInstructions from './SigningExternalInstructions.svelte';
  import SigningPublicFile from './SigningPublicFile.svelte';
  export let requests, detail, user, binding, position;
  export let disabled = false,
    onsigned;
  const operationId = crypto.randomUUID();
  let prepared = null,
    signature = '',
    mode = 'editing',
    busy = false,
    error = '',
    alive = true,
    attempted = false;
  $: statementBytes = prepared ? ownerBase64(prepared.statement_base64, 303) : null;
  $: contextKey = [user.id, detail.request_id, operationId, prepared?.statement_base64].join('|');
  $: if (!disabled && !prepared && !busy && !attempted) prepare();
  async function run(action) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await action();
    } catch (failure) {
      if (alive) error = failure.message;
    } finally {
      if (alive) busy = false;
    }
  }
  function prepare() {
    attempted = true;
    run(async () => {
      const value = await requests.prepare(operationId, {
        detail,
        position,
        accountId: user.id,
        fingerprint: binding.certificate.fingerprint,
      });
      if (alive) prepared = value;
    });
  }
  function local(value) {
    if (busy || disabled || mode !== 'editing' || value.contextKey !== contextKey) return false;
    signature = value.signatureBase64;
    return true;
  }
  function external(value) {
    if (busy || disabled || mode !== 'editing') return;
    signature = value;
  }
  async function submit() {
    if (!prepared || !signature || disabled) return;
    await run(async () => {
      mode = 'uncertain';
      try {
        const receipt = await requests.sign(detail.request_id, {
          prepared,
          signatureBase64: signature,
        });
        mode = 'done';
        onsigned(receipt);
      } catch (failure) {
        if (failure.status && failure.status < 500) mode = 'editing';
        if (failure.code === 'contract_timestamp_unavailable') mode = 'editing';
        if (failure.status && failure.status < 500 && failure.code !== 'signing_conflict')
          signature = '';
        throw failure;
      }
    });
  }
  function receipt() {
    run(async () => {
      try {
        const value = await requests.operation(detail.request_id, operationId);
        if (value?.signature?.statement_base64 !== prepared.statement_base64)
          throw new Error('El recibo no corresponde a esta firma.');
        mode = 'done';
        onsigned(value);
      } catch (failure) {
        if (failure.status === 404) {
          mode = 'editing';
          throw new Error(
            'No se registr\u00f3 la firma. Puedes enviar otra vez la misma firma cuando quieras.',
          );
        }
        throw failure;
      }
    });
  }
  onDestroy(() => (alive = false));
</script>

<section class="stack contract-sign" aria-label="Firma el contrato">
  <h3>3. Firma</h3>
  {#if disabled}<p class="hint">Completa los pasos anteriores para firmar.</p>
  {:else if !prepared}<p role="status">{busy ? 'Preparando la declaraci\u00f3n...' : ''}</p>
    {#if !busy}<button class="secondary" onclick={prepare}>Preparar la firma</button>{/if}
  {:else if mode === 'editing'}
    <p>
      Firmar&#225;s la declaraci&#243;n de esta solicitud: incluye la huella del PDF, las dos
      partes, el vencimiento y tu certificado. Selecciona tu archivo de clave y escribe su
      contrase&#241;a una sola vez.
    </p>
    {#key contextKey}<LocalSignature
        {contextKey}
        input={statementBytes}
        certificateDerBase64={binding.certificate.der_base64}
        certificateFingerprint={binding.certificate.fingerprint}
        purpose="Declaracion de firma del contrato"
        disabled={busy || disabled}
        onsignature={local}
        oninvalidate={() => (signature = '')}
      />{/key}
    <details>
      <summary>Firmar la declaraci&#243;n fuera de Qadra</summary>
      <button
        class="secondary"
        onclick={() =>
          download(
            new Blob([statementBytes], { type: 'application/octet-stream' }),
            'contract-statement.bin',
          )}>Descargar declaraci&#243;n</button
      >
      <SigningExternalInstructions kind="contract" />
      <SigningPublicFile
        label="Firma externa de la declaracion"
        disabled={busy}
        onvalue={external}
        onerror={(failure) => (error = failure.message)}
      />
    </details>
    <button class="primary" disabled={busy || !signature} onclick={submit}>Enviar firma</button>
  {:else if mode === 'uncertain'}<section class="notice stack">
      <h4>Resultado pendiente de confirmar</h4>
      <p>Consultar el recibo no vuelve a enviar la firma.</p>
      <code>{operationId}</code>
      <button class="secondary" disabled={busy} onclick={receipt}>Consultar recibo de firma</button>
    </section>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</section>
