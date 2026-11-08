<script>
  import Button from './Button.svelte';
  // One signature request seen by one of its parties. The Client signs in
  // three steps (review, confirm, sign); the firm signer reviews the PDF and
  // validates the partial acknowledgement before counter-signing. Signing
  // stays disabled until the exact PDF was loaded and its digest checked
  // and, for the firm signer, until the local validation of the partial
  // package of this request did not fail.
  // Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { onDestroy, onMount } from 'svelte';
  import { download } from '../lib/documents.mjs';
  import { signingHash } from '../lib/individual-signing-files.mjs';
  import { ownerBase64 } from '../lib/owner-certificate-binary.mjs';
  import { partialAccepted } from '../lib/contract-signing-gate.mjs';
  import {
    CLIENT_CONSENT_TEXT,
    CLIENT_SIGNED_NOTICE,
    PARTIAL_NOTICE,
    certificateExpired,
    expiryText,
    requestActions,
    requestStatusView,
  } from '../lib/contract-presentation.mjs';
  import ContractSigningReview from './ContractSigningReview.svelte';
  import ContractSignStep from './ContractSignStep.svelte';
  import ContractCloseAction from './ContractCloseAction.svelte';
  import ContractSigningGuide from './ContractSigningGuide.svelte';
  import EvidenceValidation from './EvidenceValidation.svelte';
  export let api, requests, user, requestId;
  export let onchanged = () => {},
    onnavigate = () => {};
  let detail = null,
    binding = null,
    busy = false,
    error = '',
    notice = '',
    pdfUrl = '',
    pdfBlob = null,
    confirmed = false,
    requestDigest = '',
    partialReport = null,
    alive = true;
  $: actions = detail ? requestActions(detail, user) : null;
  $: status = detail ? requestStatusView(detail.status) : null;
  $: myFingerprint = !detail
    ? ''
    : actions?.position === 1
      ? detail.client_certificate_fingerprint
      : detail.firm_certificate_fingerprint;
  $: partialChecked = actions?.position !== 2 || partialAccepted(partialReport, requestDigest);
  $: bindingProblem = !actions?.sign
    ? ''
    : !binding || binding.withdrawn_at
      ? 'missing'
      : certificateExpired(binding)
        ? 'expired'
        : binding.certificate.fingerprint !== myFingerprint
          ? 'other'
          : '';
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
  function clearPdf() {
    if (pdfUrl) URL.revokeObjectURL(pdfUrl);
    pdfUrl = '';
    pdfBlob = null;
    confirmed = false;
  }
  async function load() {
    await run(async () => {
      const value = await requests.get(requestId);
      const certificates = api.signingCertificates(user.id);
      let current = null;
      try {
        current = await certificates.current();
      } catch (failure) {
        if (failure.status !== 404) throw failure;
      } finally {
        certificates.dispose();
      }
      const digest = await signingHash(ownerBase64(value.statement_base64, 240));
      if (!alive) return;
      detail = value;
      binding = current;
      requestDigest = digest;
    });
  }
  function loadPdf() {
    run(async () => {
      clearPdf();
      const blob = await requests.content(detail.request_id, detail.document_digest);
      if (!alive) return;
      pdfBlob = blob;
      pdfUrl = URL.createObjectURL(blob);
    });
  }
  function evidence(stage) {
    run(async () => {
      const blob = await requests.evidence(detail.request_id, stage);
      download(blob, `contrato-${stage === 'partial' ? 'acuse-parcial' : 'firmado'}.zip`);
    });
  }
  async function signed(receipt) {
    clearPdf();
    notice =
      receipt.event.kind === 'client_signed'
        ? CLIENT_SIGNED_NOTICE
        : 'Firma del despacho registrada; el contrato qued\u00f3 formalizado.';
    await load();
    onchanged();
  }
  async function close(outcome, reason) {
    const value = await requests.close(detail.request_id, {
      operationId: crypto.randomUUID(),
      expectedRevision: detail.revision,
      outcome,
      reason,
    });
    notice = value.event.kind === 'rejected' ? 'Rechazaste el contrato.' : 'Declinaste la firma.';
    clearPdf();
    await load();
    onchanged();
  }
  onMount(load);
  onDestroy(() => {
    alive = false;
    clearPdf();
  });
</script>

<section class="card stack contract-signing" aria-label="Solicitud de firma" aria-busy={busy}>
  {#if !detail}<p role="status">{busy ? 'Consultando la solicitud...' : ''}</p>
  {:else}
    <div class="section-heading">
      <div>
        <span class="eyebrow">SOLICITUD DE FIRMA</span>
        <h2>{detail.title}</h2>
        <p class="hint">Vence: {expiryText(detail.expires_at_unix)}</p>
      </div>
      <span class="badge {status.tone}">{status.label}</span>
    </div>
    {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
    <!-- The standing notice is skipped while the confirmation already says it. -->
    {#if actions.position === 1 && actions.signedByMe && detail.status !== 'completed' && notice !== CLIENT_SIGNED_NOTICE}
      <p class="notice" role="status">{CLIENT_SIGNED_NOTICE}</p>
    {/if}
    {#if actions.sign && bindingProblem}
      <ContractSigningGuide problem={bindingProblem} {onnavigate} />
    {:else if actions.sign}
      <ContractSigningReview
        {detail}
        position={actions.position}
        {pdfUrl}
        {busy}
        onload={loadPdf}
        ondownload={() => download(pdfBlob, 'contrato.pdf')}
      />
      {#if actions.position === 2}<section class="stack" aria-label="Firma del Cliente">
          <h3>Comprueba la firma del Cliente</h3>
          <p>Descarga el acuse parcial y val&#237;dalo en este navegador antes de contrafirmar.</p>
          <Button variant="secondary" disabled={busy} onclick={() => evidence('partial')}
            >Descargar acuse parcial</Button
          >
          <EvidenceValidation {api} onresult={(report) => (partialReport = report)} />
          {#if !partialChecked}<p class="hint">
              Firmar se habilita cuando el acuse parcial de esta solicitud se valida sin fallas.
            </p>{/if}
        </section>{/if}
      <section class="stack" aria-label="Confirma">
        <h3>2. Confirma</h3>
        <label
          ><input type="checkbox" bind:checked={confirmed} disabled={busy || !pdfUrl} />
          {actions.position === 1
            ? CLIENT_CONSENT_TEXT
            : 'Confirmo firmar por el despacho esta versi\u00f3n exacta, despu\u00e9s de revisar la firma del Cliente.'}</label
        >
      </section>
      {#key detail.revision}<ContractSignStep
          {requests}
          {detail}
          {user}
          {binding}
          position={actions.position}
          disabled={!pdfUrl || !confirmed || !partialChecked}
          onsigned={signed}
          onstale={load}
        />{/key}
    {/if}
    {#if actions.partial && actions.position === 1}<p class="hint">{PARTIAL_NOTICE}</p>{/if}
    <div class="action-row">
      {#if actions.partial && actions.position !== 2}<Button
          variant="secondary"
          disabled={busy}
          onclick={() => evidence('partial')}>Descargar acuse parcial</Button
        >{/if}
      {#if actions.final}<Button variant="primary" disabled={busy} onclick={() => evidence('final')}
          >Descargar contrato firmado</Button
        >{/if}
      {#if actions.reject}<ContractCloseAction
          outcome="reject"
          disabled={busy}
          onconfirm={(reason) => close('reject', reason)}
        />{/if}
      {#if actions.decline}<ContractCloseAction
          outcome="decline"
          disabled={busy}
          onconfirm={(reason) => close('decline', reason)}
        />{/if}
    </div>
  {/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</section>

<style>
  .contract-signing {
    min-width: 0;
  }
</style>
