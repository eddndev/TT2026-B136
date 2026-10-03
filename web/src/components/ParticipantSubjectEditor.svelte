<script>
  import { getContext, onDestroy } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    subjectDraft,
    subjectValues,
    reviewValues,
    validateSupportSet,
    candidateKey,
  } from '../lib/typed-participant-values.mjs';
  import { typedParticipantFailure } from '../lib/typed-participant-preparation.mjs';
  import { createSubjectDraft, refreshDraftSupports } from '../lib/subject-draft.mjs';
  import { subjectCapture } from '../lib/subject-draft-values.mjs';
  import { discardOwnedSupport } from '../lib/participant-support-draft.mjs';
  import ParticipantSubjectFields from './ParticipantSubjectFields.svelte';
  import ParticipantCandidates from './ParticipantCandidates.svelte';
  import SubjectDraftComparison from './SubjectDraftComparison.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  const administration = caseState();
  const session = getContext('session-drafts');
  export let api,
    docs,
    caseId,
    onconfirmed,
    ondenied,
    pending = false;
  let dialog,
    original = null,
    draft,
    reviewed = null,
    reason = '',
    decisions = {};
  let current = null,
    comparison = null,
    busy = false,
    fieldsBusy = false;
  let candidatesBusy = false,
    error = '',
    conflict = false,
    uncertain = false;
  let exhausted = false,
    last = null,
    alive = true,
    restoreBlocked = false;
  let restoredClosed = false,
    blockedByCase = false,
    basis,
    previousBasis;
  let draftLoaded = false;
  const recovery = session
    ? createSubjectDraft({
        session,
        caseId,
        api,
        capture: () =>
          subjectCapture({
            draft,
            reason,
            decisions,
            expected: original.revision,
            uncertain,
            last,
          }),
      })
    : null;
  const admitted = () => alive && (!recovery || recovery.admitted());
  $: basis = JSON.stringify(draft);
  $: if (basis !== previousBasis) {
    previousBasis = basis;
    reviewed = null;
  }
  $: pending = busy || fieldsBusy || candidatesBusy;
  $: readonly = restoreBlocked || restoredClosed || $administration.closed;
  $: staleDecisions = Object.keys(decisions).filter(
    (key) => reviewed && !reviewed.candidates.some((row) => candidateKey(row.reference) === key),
  );
  $: if (blockedByCase && !$administration.closed) {
    blockedByCase = restoredClosed = false;
    error = '';
  }
  function supportContext(key = null) {
    if (!recovery || !original || restoreBlocked) return null;
    const id = original.id;
    return recovery.supportContext(
      id,
      key ? ['decisions', 'support'] : ['identity_support'],
      key,
      () => original?.id === id && !restoreBlocked && (!key || !!decisions[key]),
      (failure) => {
        recovery.discard(failure, id);
        original = draft = null;
        error = failure.message;
        ondenied(failure);
      },
    );
  }
  function redeclare(key) {
    const context = supportContext(key);
    if (context) discardOwnedSupport(session, context);
  }
  function supportDenied(failure) {
    if (!admitted()) return;
    reviewed = null;
    error = failure.message;
    if (failure.code === 'case_not_found') {
      recovery?.discard(failure, original?.id);
      ondenied(failure);
    }
  }
  export function hasSuspendedDraft(id) {
    return recovery?.pending(id) ?? false;
  }
  export async function open(record) {
    if (pending || !admitted() || (dialog.open && !restoreBlocked)) return;
    const retained = restoreBlocked && draftLoaded && original?.id === record.id;
    const saved = recovery?.pending(record.id);
    if ($administration.closed && !saved && !retained) return;
    restoreBlocked = true;
    error = '';
    if (!retained) {
      draftLoaded = false;
      original = { id: record.id, revision: record.revision };
      draft = subjectDraft();
      reviewed = current = comparison = last = null;
      reason = '';
      decisions = {};
      uncertain = conflict = exhausted = restoredClosed = false;
    }
    if (!dialog.open) dialog.showModal();
    busy = true;
    try {
      if (saved && !retained) {
        const result = await recovery.restore(record.id, (value, context) => {
          ({ draft, reason, decisions, uncertain, last } = value);
          original = { ...context.record, revision: value.expected };
          restoredClosed = context.status === 'closed';
          if (context.record.revision !== value.expected) {
            current = context.record;
            conflict = true;
          }
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el borrador de identidad.');
      } else {
        const context = recovery
          ? await recovery.freshContext(record.id)
          : { record, status: $administration.closed ? 'closed' : 'active' };
        if (!admitted() || !context) return;
        if (retained) {
          if (context.record.revision < original.revision)
            throw new Error('La revisi\u00f3n actual no confirma la base del borrador.');
          if (context.record.revision !== original.revision) {
            current = context.record;
            conflict = true;
          }
        } else {
          original = context.record;
          draft = subjectDraft(original.values);
        }
        restoredClosed = context.status === 'closed';
        recovery?.register(original.id, original.revision);
      }
      draftLoaded = true;
      const rejected = await refreshDraftSupports({ draft, decisions, docs, caseId, admitted });
      if (!admitted()) return;
      draft = draft;
      decisions = decisions;
      restoreBlocked = false;
      if (rejected)
        error = 'Un soporte ya no est\u00e1 disponible. Selecciona otra versi\u00f3n autorizada.';
    } catch (failure) {
      if (!admitted()) return;
      error = failure.message;
      if ([403, 404].includes(failure.status)) {
        recovery?.discard(failure, record.id);
        original = draft = null;
        ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function release() {
    recovery?.close(original?.id);
    dialog.close();
    original = draft = reviewed = last = null;
    draftLoaded = false;
  }
  function close() {
    if (!pending) release();
  }
  async function work(operation) {
    if (pending || !admitted() || restoreBlocked) return;
    busy = true;
    error = '';
    try {
      await operation();
    } catch (failure) {
      if (admitted()) {
        error =
          uncertain || failure.status || failure.code
            ? typedParticipantFailure(failure)
            : failure.message;
        conflict = failure.code === 'subject_revision_conflict' || conflict;
        exhausted = failure.code?.endsWith('_revision_exhausted');
        if (failure.code === 'case_closed') blockedByCase = restoredClosed = true;
        if ([403, 404].includes(failure.status)) {
          recovery?.discard(failure, original?.id);
          ondenied(failure);
        }
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function review() {
    if (readonly || uncertain || conflict) return;
    return work(async () => {
      const values = subjectValues(draft);
      validateSupportSet(values);
      const result = await api.reviewSubject(original.id, original.revision, values);
      if (admitted()) reviewed = result;
    });
  }
  function save() {
    if (readonly || uncertain || conflict || !reviewed) return;
    return work(async () => {
      const values = subjectValues(draft),
        review = reviewValues(reviewed, reason, decisions);
      validateSupportSet(values, review);
      last = { expected: original.revision, values, review };
      uncertain = true;
      try {
        const result = await api.replaceSubject(original.id, last.expected, values, review);
        if (!admitted()) return;
        release();
        await onconfirmed(result);
      } catch (failure) {
        if (admitted()) uncertain = !failure.status || failure.status >= 500;
        throw failure;
      }
    });
  }
  function refresh(currentHead = false) {
    return work(async () => {
      try {
        const result =
          uncertain && !currentHead
            ? await api.subjectRevision(original.id, last.expected + 1)
            : await api.subject(original.id);
        if (admitted()) current = result;
      } catch (failure) {
        if (
          uncertain &&
          !currentHead &&
          failure.status === 404 &&
          failure.code === 'subject_not_found'
        ) {
          if (admitted()) {
            current = null;
            error =
              'Esta consulta no encontr\u00f3 la revisi\u00f3n enviada. Conservamos el formulario; puedes consultar de nuevo o revisar la identidad actual antes de decidir.';
          }
          return;
        }
        throw failure;
      }
    });
  }
  function useCurrent() {
    if (!current || pending || readonly || !admitted()) return;
    original = current;
    current = null;
    uncertain = conflict = false;
    reviewed = last = null;
    recovery?.register(original.id, original.revision);
  }
  function chooseCandidate(row) {
    return work(async () => {
      const result =
        row.reference.kind === 'subject'
          ? await api.subjectRevision(row.reference.id, row.reference.revision)
          : await api.participantRevision(row.reference.id, row.reference.revision);
      if (admitted()) comparison = result;
    });
  }
  onDestroy(() => {
    alive = false;
    pending = false;
    recovery?.dispose();
  });
</script>

<dialog
  class="upload-dialog participant-dialog"
  aria-busy={pending}
  bind:this={dialog}
  aria-labelledby="participant-subject-editor"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <h2 id="participant-subject-editor">Editar identidad del expediente</h2>
    <button class="text-button" disabled={pending} onclick={close}>Cerrar identidad</button>
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if restoreBlocked && original}<button
      class="secondary"
      disabled={pending}
      onclick={() => open(original)}>Volver a consultar el contexto</button
    >{/if}
  {#if original}<div class="stack">
      <p>
        Este cambio crea una revisi&#243;n de identidad. Las fichas ya registradas conservan la
        revisi&#243;n que ten&#237;an vinculada y sus firmas anteriores.
      </p>
      {#key restoreBlocked}<ParticipantSubjectFields
          bind:draft
          {docs}
          {caseId}
          ondenied={supportDenied}
          fixedKind
          draftContext={supportContext()}
          disabled={busy || candidatesBusy || readonly}
          bind:pending={fieldsBusy}
        />{/key}
      <button
        class="secondary"
        disabled={pending || uncertain || conflict || exhausted || readonly}
        onclick={review}>Revisar coincidencias de identidad</button
      >
      {#if reviewed}<ParticipantCandidates
          result={reviewed}
          bind:reason
          bind:decisions
          {docs}
          {caseId}
          ondenied={supportDenied}
          {supportContext}
          onredeclare={redeclare}
          choiceLabel="Consultar candidato"
          onchoose={chooseCandidate}
          disabled={busy || fieldsBusy || readonly}
          bind:pending={candidatesBusy}
        />{/if}
      {#each staleDecisions as key}<section aria-label="Decision anterior sin aplicar">
          <p>
            Esta decisi&#243;n corresponde a otra revisi&#243;n del candidato. No se aplica a las
            coincidencias actuales.
          </p>
          <label
            >Motivo anterior conservado<textarea readonly value={decisions[key].reason}
            ></textarea></label
          >
          {#if decisions[key].support}<label
              >Localizador anterior conservado<textarea
                readonly
                value={decisions[key].support.locator}></textarea></label
            >{/if}
        </section>{/each}
      <SubjectDraftComparison
        {comparison}
        {current}
        {uncertain}
        {conflict}
        {pending}
        {readonly}
        {refresh}
        {useCurrent}
      />
      <CaseClosedNotice />
      <div class="dialog-actions">
        <button class="secondary" disabled={pending} onclick={close}>Cancelar</button>
        <button
          class="primary"
          disabled={pending || !reviewed || uncertain || conflict || exhausted || readonly}
          onclick={save}>Guardar revisi&#243;n de identidad</button
        >
      </div>
    </div>
  {:else}<p role="status">Consultando la identidad y el expediente actuales...</p>
    <button class="secondary" disabled={pending} onclick={close}>Cancelar</button>{/if}
</dialog>
