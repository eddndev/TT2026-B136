<script>
  import { getContext, onDestroy } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { createSigningDraft } from '../lib/individual-signing-draft.mjs';
  import { checkedSigningPdf } from '../lib/individual-signing-files.mjs';
  import { decodeSigningStatement } from '../lib/individual-signing-values.mjs';
  import { signingConsent } from '../lib/individual-signing-responses.mjs';
  import { ownerBase64 } from '../lib/owner-certificate-binary.mjs';
  import { download } from '../lib/documents.mjs';
  import IndividualSigningEditor from './IndividualSigningEditor.svelte';
  import SigningConsentDetails from './SigningConsentDetails.svelte';
  export let api, user, document, onupdate, ondenied;
  export let busy = '',
    disabled = false,
    active = false;
  const session = getContext('session-drafts'),
    administration = caseState();
  const context = {
    accountId: user.id,
    caseId: document.case_id,
    documentId: document.id,
    version: document.version,
    digest: document.digest,
  };
  const blank = () => ({
    operationId: '',
    prepared: null,
    pdfSignature: '',
    consentSignature: '',
    mode: 'editing',
  });
  let state = blank(),
    saved = null,
    binding = null,
    receipt = null;
  let alive = true,
    opened = false,
    error = '',
    pdfUrl = '',
    pdfBlob = null,
    pdfRevision = 0,
    consentRevision = 0,
    reviewed = false,
    confirmed = false;
  const draft = createSigningDraft(session, context, 'document', () => ({
    state,
    digest: context.digest,
  }));
  saved = draft.pending();
  const personal = () => api.individualSigning(context.accountId, context.digest);
  const admitted = () =>
    alive &&
    draft.admitted() &&
    document.case_id === context.caseId &&
    document.id === context.documentId &&
    document.version === context.version &&
    document.digest === context.digest;
  $: pdfContext = [
    context.accountId,
    context.caseId,
    context.documentId,
    context.version,
    context.digest,
    state.operationId,
    binding?.binding_id,
    binding?.certificate.fingerprint,
    pdfRevision,
  ].join('|');
  $: consentContext = [pdfContext, consentRevision, state.prepared?.consent_base64].join('|');
  function acceptPdfSignature(value, contextKey = null) {
    if (
      !admitted() ||
      busy ||
      disabled ||
      $administration.closed ||
      !reviewed ||
      !pdfBlob ||
      state.prepared ||
      state.mode !== 'editing'
    )
      return false;
    if (contextKey !== null && contextKey !== pdfContext) return false;
    if (contextKey === null) pdfRevision++;
    state = { ...state, pdfSignature: value, prepared: null, consentSignature: '' };
    confirmed = false;
    error = '';
    return true;
  }
  function acceptConsentSignature(value, contextKey = null) {
    if (
      !admitted() ||
      busy ||
      disabled ||
      $administration.closed ||
      !reviewed ||
      !state.prepared ||
      state.mode !== 'editing'
    )
      return false;
    if (contextKey !== null && (contextKey !== consentContext || !confirmed)) return false;
    if (contextKey === null) consentRevision++;
    state = { ...state, consentSignature: value };
    if (contextKey === null) confirmed = false;
    error = '';
    return true;
  }
  async function fresh(requireAvailable = false) {
    const principal = await api.signingIdentity();
    if (!admitted()) return false;
    if (principal.id !== context.accountId || principal.role !== 'litigator')
      throw Object.assign(new Error('La cuenta ya no admite firma personal.'), { status: 403 });
    const value = await api.detail();
    if (!admitted()) return false;
    if (
      value.case_id !== context.caseId ||
      value.id !== context.documentId ||
      value.version !== context.version ||
      value.digest !== context.digest
    )
      throw new Error('La version cambio. Abre su detalle otra vez.');
    if (requireAvailable && (value.sealed || $administration.closed))
      throw new Error('La version ya no admite una firma nueva.');
    return true;
  }
  async function run(label, action) {
    if (busy || disabled || !admitted()) return;
    busy = label;
    error = '';
    try {
      await action();
    } catch (failure) {
      if ([400, 422].includes(failure.status) && state.mode === 'uncertain') {
        state = { ...state, mode: 'editing', consentSignature: '' };
        confirmed = false;
      }
      if (alive)
        error =
          failure.status === 401
            ? 'Vuelve a ingresar y recupera el intento para consultar su recibo.'
            : failure.status === 404
              ? 'No se encontro el recibo. Conserva la operacion y consulta nuevamente.'
              : state.mode === 'uncertain'
                ? 'No se confirmo el resultado. Consulta el recibo de esta misma operacion.'
                : failure.message;
      if (failure.status === 403) {
        draft.close();
        ondenied(failure);
      }
    } finally {
      if (alive) busy = '';
    }
  }
  function open() {
    if (saved || document.sealed || $administration.closed) return;
    run('personal-open', async () => {
      if (!(await fresh(true))) return;
      const certificates = api.signingCertificate(context.accountId);
      try {
        binding = await certificates.current();
      } finally {
        certificates.dispose();
      }
      if (!admitted()) return;
      if (!binding || binding.withdrawn_at)
        throw new Error('Primero registra tu certificado en Mi firma documental.');
      state = { ...blank(), operationId: crypto.randomUUID() };
      draft.register();
      opened = active = true;
    });
  }
  function clearPdf() {
    if (pdfUrl) URL.revokeObjectURL(pdfUrl);
    pdfUrl = '';
    pdfBlob = null;
    pdfRevision++;
    reviewed = confirmed = false;
  }
  function getPdf(exportFile = false) {
    run('personal-pdf', async () => {
      clearPdf();
      state = { ...state, pdfSignature: '', prepared: null, consentSignature: '' };
      const result = await api.content(context.digest);
      const blob = await checkedSigningPdf(result.blob, context.digest);
      if (!admitted()) return;
      pdfBlob = blob;
      pdfUrl = URL.createObjectURL(blob);
      if (exportFile) download(blob, 'document.pdf');
    });
  }
  function prepare() {
    if (!reviewed || !pdfUrl || !state.pdfSignature || state.prepared || state.mode !== 'editing')
      return;
    run('personal-prepare', async () => {
      if (!(await fresh(true))) return;
      const value = await personal().prepare(state.operationId, state.pdfSignature);
      if (!admitted()) return;
      const canonical = decodeSigningStatement(value.consent_base64, 'consent');
      const original = decodeSigningStatement(binding.statement_base64, 'binding');
      if (
        canonical.bindingId !== original.bindingId ||
        canonical.leafFingerprint !== original.leafFingerprint ||
        canonical.deploymentId !== original.deploymentId ||
        canonical.rootFingerprint !== original.rootFingerprint ||
        canonical.trustRevision < original.trustRevision
      )
        throw new Error('El vinculo cambio. Prepara una nueva revision del PDF.');
      state = { ...state, prepared: value, consentSignature: '' };
      confirmed = false;
    });
  }
  async function complete(value) {
    if (!admitted()) return;
    receipt = value;
    state = { ...state, mode: 'complete' };
    draft.close();
    saved = null;
    opened = active = false;
    clearPdf();
    await onupdate({ ...document, sealed: true, report: null });
  }
  function submit() {
    if (
      !reviewed ||
      !confirmed ||
      !state.prepared ||
      !state.consentSignature ||
      state.mode !== 'editing'
    )
      return;
    run('personal-submit', async () => {
      if (!(await fresh(true))) return;
      const input = Object.freeze({
        consent_base64: state.prepared.consent_base64,
        pdf_signature_base64: state.pdfSignature,
        consent_signature_base64: state.consentSignature,
      });
      state = { ...state, mode: 'uncertain' };
      await complete(await personal().submit(input));
    });
  }
  function recover() {
    run('personal-recover', async () => {
      const result = await draft.restore(
        () => fresh(),
        (value) => {
          if (value.state.mode === 'uncertain') {
            signingConsent(value.state.prepared, context, value.state.operationId);
            ownerBase64(value.state.pdfSignature, 384);
            ownerBase64(value.state.consentSignature, 384);
            state = value.state;
          } else state = blank();
          opened = state.mode === 'uncertain';
          active = opened;
          clearPdf();
        },
      );
      if (result.status !== 'restored') throw new Error('No se pudo recuperar este intento.');
      saved = null;
    });
  }
  function checkReceipt() {
    run('personal-receipt', async () => {
      if (!(await fresh())) return;
      await complete(await personal().receipt(state.operationId, state.prepared));
    });
  }
  function cancel() {
    if (busy || state.mode === 'uncertain') return;
    draft.close();
    saved = null;
    state = blank();
    opened = active = false;
    clearPdf();
  }
  onDestroy(() => {
    draft.dispose(state.mode === 'uncertain');
    alive = false;
    clearPdf();
    active = false;
  });
</script>

{#if saved}<section class="notice stack">
    <p>Hay un intento de firma conservado para esta version.</p>
    <button class="secondary" disabled={!!busy || disabled} onclick={recover}
      >Recuperar intento de firma</button
    >
  </section>
{:else if !opened && !receipt && !document.sealed}
  <button class="secondary" disabled={!!busy || disabled || $administration.closed} onclick={open}
    >Firmar personalmente</button
  >
{/if}
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if opened && state.mode === 'editing'}
  <IndividualSigningEditor
    {state}
    {pdfUrl}
    {pdfBlob}
    {pdfContext}
    {consentContext}
    certificate={binding.certificate}
    signatureDisabled={$administration.closed}
    busy={!!busy || disabled}
    bind:reviewed
    bind:confirmed
    onreview={() => getPdf()}
    ondownload={() => getPdf(true)}
    onprepare={prepare}
    onsubmit={submit}
    oncancel={cancel}
    onpdfsignature={(value) => acceptPdfSignature(value)}
    onconsentsignature={(value) => acceptConsentSignature(value)}
    onlocalpdfsignature={(value) => acceptPdfSignature(value.signatureBase64, value.contextKey)}
    onlocalconsentsignature={(value) =>
      acceptConsentSignature(value.signatureBase64, value.contextKey)}
    onlocalpdfinvalidate={(contextKey) => acceptPdfSignature('', contextKey)}
    onlocalconsentinvalidate={(contextKey) => acceptConsentSignature('', contextKey)}
    onerror={(failure) => (error = failure.message)}
  />
{/if}
{#if state.mode === 'uncertain'}<section class="notice stack">
    <h3>Resultado pendiente de confirmar</h3>
    <p>Conserva esta operacion. Consultar el recibo no vuelve a enviar la firma.</p>
    <code>{state.operationId}</code>
    <button class="secondary" disabled={!!busy || disabled} onclick={checkReceipt}
      >Consultar recibo de firma</button
    >
  </section>{/if}
{#if receipt}<section class="card stack">
    <h3>Firma personal aceptada</h3>
    <SigningConsentDetails value={receipt} />
    <button
      class="secondary"
      onclick={() =>
        download(
          new Blob([JSON.stringify(receipt)], { type: 'application/json' }),
          'document-signing-receipt.json',
        )}>Descargar recibo de firma</button
    >
  </section>{/if}
