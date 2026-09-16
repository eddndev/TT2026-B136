<script>
  import { onDestroy, tick } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { needsCredential } from '../lib/typed-participant-fields.mjs';
  import {
    subjectDraft,
    roleDraft,
    reviewValues,
    validateSupportSet,
  } from '../lib/typed-participant-values.mjs';
  import { proposalRequest, resolveCandidate } from '../lib/typed-participant-proposal.mjs';
  import {
    base64Bytes,
    bytesBase64,
    readSubmission,
    typedParticipantFailure,
  } from '../lib/typed-participant-preparation.mjs';
  import TypedParticipantForm from './TypedParticipantForm.svelte';
  import ParticipantCredential from './ParticipantCredential.svelte';
  import ParticipantCandidates from './ParticipantCandidates.svelte';
  import TypedParticipantNotice from './TypedParticipantNotice.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  const administration = caseState();
  export let api, manualApi, docs, caseId, onconfirmed, onobserved, ondenied;
  let dialog,
    credential,
    original = null,
    subject = subjectDraft(),
    selected = null,
    role = roleDraft();
  let review = null,
    decisions = {},
    reason = '',
    prepared = null,
    preparation = null,
    lastSubmission = null;
  let value = null,
    busy = false,
    formBusy = false,
    credentialBusy = false,
    candidatesBusy = false,
    directoryBusy = false,
    error = '',
    alive = true,
    opened = false;
  let uncertain = false,
    checkedAbsent = false,
    conflict = false,
    current = null,
    exhausted = false,
    generation = 0;
  let previousBasis, previousCertificate, basis, reviewBasis, previousReviewBasis;
  $: reviewBasis = JSON.stringify({ reason, decisions });
  $: if (reviewBasis !== previousReviewBasis) {
    previousReviewBasis = reviewBasis;
    prepared = null;
    preparation = null;
  }
  $: basis = JSON.stringify({ subject, selected, role, base: original?.revision });
  $: if (basis !== previousBasis) {
    previousBasis = basis;
    invalidate();
  }
  $: if (value?.certificate?.blob !== previousCertificate) {
    previousCertificate = value?.certificate?.blob;
    invalidate();
  }
  $: pending = busy || formBusy || credentialBusy || candidatesBusy || directoryBusy;
  $: closed = $administration.closed;
  $: title = original
    ? original.profile
      ? 'Editar ficha tipificada'
      : 'Completar perfil de participante'
    : 'Agregar participante tipificado';
  $: natural = (selected?.values.kind || subject.kind) === 'natural_person';
  function invalidate() {
    review = null;
    prepared = null;
    preparation = null;
  }
  export async function open(record = null) {
    if (closed) return;
    generation++;
    original = record;
    selected = record?.subject || null;
    subject = subjectDraft();
    role = roleDraft('', record?.profile ? record : undefined);
    invalidate();
    decisions = {};
    reason = '';
    lastSubmission = null;
    uncertain = false;
    conflict = false;
    current = null;
    exhausted = false;
    error = '';
    opened = true;
    await tick();
    dialog.showModal();
  }
  export function close() {
    if (pending) return;
    release();
  }
  function release() {
    generation++;
    opened = false;
    dialog.close();
    selected = null;
    subject = subjectDraft();
    role = roleDraft();
    invalidate();
    lastSubmission = null;
  }
  async function work(operation) {
    if (pending || !alive) return;
    const ticket = generation;
    busy = true;
    error = '';
    try {
      await operation(() => alive && ticket === generation);
    } catch (failure) {
      if (alive && ticket === generation) {
        error =
          uncertain || failure.status || failure.code
            ? typedParticipantFailure(failure)
            : failure.message;
        exhausted = failure.code?.endsWith('_revision_exhausted');
        conflict = ['participant_revision_conflict', 'subject_revision_conflict'].includes(
          failure.code,
        );
        if (
          [403, 404].includes(failure.status) &&
          !['subject_not_found', 'participant_credential_not_found'].includes(failure.code)
        )
          ondenied(failure);
      }
    } finally {
      if (alive && ticket === generation) busy = false;
    }
  }
  function reviewIdentity() {
    if (closed || uncertain || conflict) return;
    return work(async (valid) => {
      const certificate =
        natural && value?.certificate ? await bytesBase64(value.certificate.blob) : null;
      const result = await api.review(
        proposalRequest({ selected, subject, role, original, natural }, certificate),
      );
      if (!valid()) return;
      review = result;
      prepared = null;
      decisions = {};
      preparation = null;
    });
  }
  function choose(row) {
    return work(async (valid) => {
      const result = await resolveCandidate(row, { api, manualApi, original, selected });
      if (!valid()) return;
      original = result.original;
      selected = result.selected;
      invalidate();
      error = 'Candidato consultado. Revisa de nuevo la identidad con esta selecci\u00f3n.';
    });
  }
  function prepare() {
    if (closed || uncertain || conflict || !review) return;
    return work(async (valid) => {
      const identityReview = reviewValues(review, reason, decisions);
      validateSupportSet(review.proposal, selected?.values, identityReview);
      const certificate =
        natural && value?.certificate ? await bytesBase64(value.certificate.blob) : null;
      const request = {
        proposal: review.proposal,
        review: identityReview,
        certificate_base64: certificate,
      };
      const ticket = certificate ? credential.beginPreparation() : null;
      try {
        const result = await api.prepare(request);
        if (!valid()) return;
        if (
          result.declaration &&
          !credential.acceptPreparation(
            ticket,
            base64Bytes(result.declaration.bytes_base64),
            new TextEncoder().encode(JSON.stringify(result, null, 2)),
          )
        )
          return;
        prepared = result;
        preparation = request;
      } catch (failure) {
        if (ticket !== null) credential.failPreparation(ticket, typedParticipantFailure(failure));
        throw failure;
      }
    });
  }
  async function send(bundle, valid) {
    try {
      const record = await api.commit(bundle.request);
      if (!valid()) return;
      uncertain = false;
      await onconfirmed(record);
      if (valid()) {
        busy = false;
        release();
      }
    } catch (failure) {
      if (valid()) {
        uncertain = !failure.status || failure.status >= 500;
        checkedAbsent = false;
      }
      throw failure;
    }
  }
  function submit() {
    if (
      closed ||
      uncertain ||
      conflict ||
      exhausted ||
      !prepared ||
      (prepared.declaration && !value?.ready)
    )
      return;
    return work(async (valid) => {
      const signature_base64 = prepared.declaration
        ? await bytesBase64(value.signature.blob)
        : null;
      lastSubmission = { prepared, request: { prepared: preparation, signature_base64 } };
      await send(lastSubmission, valid);
    });
  }
  function reconcile() {
    return work(async (valid) => {
      checkedAbsent = false;
      const result = await readSubmission(api, lastSubmission, valid);
      if (!valid()) return;
      if (result.state === 'absent') {
        checkedAbsent = true;
        error =
          'Esta consulta no encontr\u00f3 la revisi\u00f3n enviada. Puedes consultar de nuevo o reenviar expl\u00edcitamente el mismo registro, sin crear otros identificadores.';
        return;
      }
      const record = result.record;
      if (result.state === 'matched') {
        uncertain = false;
        await onconfirmed(record);
        if (valid()) {
          busy = false;
          release();
        }
      } else {
        current = { record, exact: true };
        error =
          'La revisi\u00f3n consultada no corresponde al env\u00edo conservado. Compara los datos; no se reenviar\u00e1 autom\u00e1ticamente.';
      }
    });
  }
  function refresh() {
    return work(async (valid) => {
      const record = original ? await manualApi.get(original.id) : null;
      const identity = selected ? await api.subject(selected.id) : null;
      if (!valid()) return;
      current = { record, identity };
      error = '';
      if (record) await onobserved(record);
    });
  }
  function useCurrent() {
    if (current?.record) original = current.record;
    if (current?.identity) selected = current.identity;
    conflict = false;
    uncertain = false;
    current = null;
    invalidate();
  }
  onDestroy(() => {
    alive = false;
    generation++;
    opened = false;
    lastSubmission = null;
  });
</script>

<dialog
  class="upload-dialog participant-dialog"
  aria-busy={pending}
  bind:this={dialog}
  aria-labelledby="typed-participant-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <div>
      <span class="eyebrow">DIRECTORIO DEL EXPEDIENTE</span>
      <h2 id="typed-participant-title">{title}</h2>
    </div>
    <button class="text-button" disabled={pending} onclick={close}>Cerrar ficha</button>
  </div>
  {#if opened}<div class="stack">
      <p>
        Registra una identidad y su participaci&#243;n en el expediente. Esto no crea una cuenta ni
        acredita efectos jur&#237;dicos autom&#225;ticos.
      </p>
      <TypedParticipantForm
        {api}
        {docs}
        {caseId}
        bind:subject
        bind:selected
        bind:role
        fixedSubject={!!original?.profile}
        {ondenied}
        disabled={busy || credentialBusy || candidatesBusy || directoryBusy || closed}
        bind:pending={formBusy}
      />
      {#if natural}<ParticipantCredential
          bind:this={credential}
          mandatory={needsCredential(role.profile.kind)}
          scopeKey={`${caseId}:${generation}:${selected?.id || 'new-identity'}`}
          basisKey={`${basis}:${reviewBasis}`}
          disabled={busy || formBusy || candidatesBusy || directoryBusy || closed}
          bind:pending={credentialBusy}
          bind:value
        />{/if}
      <button
        type="button"
        class="secondary"
        disabled={pending || closed || uncertain || conflict || exhausted}
        onclick={reviewIdentity}>Revisar identidad y coincidencias</button
      >
      {#if review}<ParticipantCandidates
          result={review}
          bind:decisions
          bind:reason
          {docs}
          {caseId}
          {ondenied}
          onchoose={choose}
          disabled={busy || formBusy || credentialBusy || directoryBusy || closed}
          bind:pending={candidatesBusy}
        /><button
          type="button"
          class="secondary"
          disabled={pending || closed || uncertain || exhausted}
          onclick={prepare}>Preparar registro</button
        >{/if}
      <TypedParticipantNotice
        {prepared}
        {error}
        {conflict}
        {current}
        {uncertain}
        {checkedAbsent}
        {pending}
        {closed}
        onrefresh={refresh}
        onusecurrent={useCurrent}
        onreconcile={reconcile}
        onresend={() => work((valid) => send(lastSubmission, valid))}
        api={manualApi}
        kind={role.profile.kind}
        {ondenied}
        directoryDisabled={busy || formBusy || credentialBusy || candidatesBusy}
        bind:directoryBusy
      />
      <CaseClosedNotice />
      <div class="dialog-actions">
        <button class="secondary" disabled={pending} onclick={close}>Cancelar</button><button
          class="primary"
          disabled={pending ||
            closed ||
            uncertain ||
            conflict ||
            exhausted ||
            !prepared ||
            (!!prepared.declaration && !value?.ready)}
          onclick={submit}>{busy ? 'Procesando...' : 'Confirmar registro'}</button
        >
      </div>
    </div>{/if}
</dialog>
