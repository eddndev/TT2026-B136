<script>
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { getContext, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import ParticipantFields from './ParticipantFields.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import { participantDraft, participantFailure } from '../lib/participants.mjs';
  import { createManualParticipantDraft } from '../lib/manual-participant-draft.mjs';
  export let api;
  export let caseId;
  export let disabled = false;
  export let ondraftchange = () => {};
  export let onconfirmed;
  export let onobserved;
  export let ondenied;
  let dialog;
  let fields;
  let original = null;
  let draft = participantDraft();
  let candidate = null;
  let busy = false;
  let submitting = false;
  let conflict = false;
  let exhausted = false;
  let error = '';
  let blockedByCase = false;
  let originalExpectedRevision = null,
    restoreBlocked = false,
    restoredClosed = false,
    typedCurrent = false,
    restored = false,
    unconfirmed = false,
    needsReview = false,
    reviewAccepted = false,
    directoryReviewed = false,
    directory = [];
  const sessionDrafts = getContext('session-drafts');
  const recovery = sessionDrafts
    ? createManualParticipantDraft({
        session: sessionDrafts,
        caseId,
        api,
        capture: () => ({ draft, originalExpectedRevision, unconfirmed }),
      })
    : null;
  const admitted = () => (recovery ? recovery.admitted() : alive);
  $: fieldsBlocked =
    busy || restoreBlocked || restoredClosed || typedCurrent || $administration.closed;
  $: submitBlocked =
    fieldsBlocked ||
    disabled ||
    exhausted ||
    (conflict && !candidate) ||
    (needsReview && !reviewAccepted) ||
    (unconfirmed && !original && !directoryReviewed);
  $: if (blockedByCase && !$administration.closed) {
    error = '';
    blockedByCase = false;
    restoredClosed = false;
  }
  let alive = true;
  export function hasSuspendedDraft(id = null) {
    return recovery?.pending(id) ?? false;
  }
  export function pendingDraftIds() {
    return recovery?.pendingIds() ?? [];
  }
  export function discardDraft(failure) {
    recovery?.discard(failure, original?.id ?? null);
  }
  function deny(failure) {
    if (![403, 404].includes(failure.status)) return;
    discardDraft(failure);
    restoreBlocked = true;
    ondenied(failure, original?.id ?? null);
  }
  function resetReview() {
    needsReview = reviewAccepted = directoryReviewed = false;
    directory = [];
    candidate = null;
  }
  export async function open(record = null) {
    if (busy || disabled || !admitted() || (dialog.open && !restoreBlocked)) return;
    const id = record?.id ?? null,
      saved = recovery?.pending(id);
    if ($administration.closed && !saved) return;
    original = record;
    originalExpectedRevision = record?.revision ?? null;
    draft = participantDraft();
    resetReview();
    restoreBlocked = !!recovery;
    restoredClosed = typedCurrent = restored = unconfirmed = false;
    conflict = false;
    exhausted = false;
    error = '';
    fields?.reset();
    if (!dialog.open) dialog.showModal();
    busy = true;
    try {
      if (saved) {
        const result = await recovery.restore(id, (value, context) => {
          ({ draft, originalExpectedRevision, unconfirmed } = value);
          restored = true;
          restoredClosed = context.status === 'closed';
          typedCurrent = !!context.record?.profile;
          candidate = context.record;
          needsReview =
            unconfirmed || (!!candidate && candidate.revision !== originalExpectedRevision);
          if (!needsReview && !typedCurrent && !restoredClosed) candidate = null;
          conflict = needsReview && id !== null;
          restoreBlocked = false;
        });
        if (admitted() && result.status !== 'restored')
          error = 'No se pudo recuperar el borrador. Vuelve a consultar el contexto.';
      } else {
        const context = recovery
          ? await recovery.freshContext(id)
          : {
              record,
              status: $administration.closed ? 'closed' : 'active',
            };
        if (!context || !admitted()) return;
        original = context.record;
        originalExpectedRevision = original?.revision ?? null;
        draft = participantDraft(original || {});
        restoredClosed = context.status === 'closed';
        typedCurrent = !!original?.profile;
        if (!restoredClosed && !typedCurrent) recovery?.register(id, originalExpectedRevision);
        restoreBlocked = false;
      }
    } catch (failure) {
      if (alive) {
        error = failure.message;
        deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function close() {
    if (busy) return;
    recovery?.close();
    ondraftchange();
    dialog.close();
    draft = participantDraft();
    original = null;
    originalExpectedRevision = null;
    restoreBlocked = restoredClosed = typedCurrent = restored = unconfirmed = false;
    resetReview();
    fields?.reset();
  }
  async function refresh() {
    if (busy || disabled || restoreBlocked || !admitted()) return;
    busy = true;
    reviewAccepted = directoryReviewed = false;
    needsReview = restored || unconfirmed;
    candidate = null;
    try {
      if (!original) {
        directory = [];
        const context = await recovery.currentDirectory();
        if (!context || !admitted()) return;
        restoredClosed = context.status === 'closed';
        directory = context.participants;
        directoryReviewed = true;
        return;
      }
      const context = recovery
        ? await recovery.freshContext(original.id)
        : {
            record: await api.get(original.id),
            status: $administration.closed ? 'closed' : 'active',
          };
      if (!context || !admitted()) return;
      if (context.record.revision < originalExpectedRevision)
        throw new Error('La revisi\u00f3n actual no confirma la base del borrador.');
      candidate = context.record;
      restoredClosed = context.status === 'closed';
      typedCurrent = !!candidate.profile;
      error = '';
      await onobserved(candidate);
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (alive) {
        error = failure.message;
        deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || submitBlocked || !admitted()) return;
    let values;
    try {
      values = fields.values();
    } catch {
      error = '';
      return;
    }
    const expected = candidate?.revision ?? originalExpectedRevision;
    const previousReview = needsReview;
    busy = submitting = true;
    error = '';
    unconfirmed = needsReview = true;
    candidate = null;
    reviewAccepted = directoryReviewed = false;
    try {
      const record = original
        ? await api.replace(original.id, expected, {
            ...values,
            directory_status: draft.directory_status,
          })
        : await api.create(values);
      if (!admitted()) return;
      unconfirmed = false;
      busy = false;
      close();
      await onconfirmed(record);
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (!alive) return;
      if (!dialog.open) return;
      if (failure.code === 'case_closed') restoredClosed = true;
      if (failure.code === 'participant_profile_required') typedCurrent = true;
      unconfirmed = !failure.status || failure.status >= 500;
      conflict = failure.code === 'participant_revision_conflict';
      exhausted = failure.code === 'participant_revision_exhausted';
      candidate = null;
      needsReview = (restored && conflict) || previousReview || unconfirmed;
      error = conflict
        ? 'Los datos cambiaron mientras editabas. Consulta los valores actuales y revisa tu formulario.'
        : participantFailure(failure);
      deny(failure);
    } finally {
      if (alive) busy = submitting = false;
    }
  }
  onDestroy(() => {
    alive = false;
    recovery?.dispose();
  });
</script>

<dialog
  class="upload-dialog participant-dialog"
  bind:this={dialog}
  aria-labelledby="participant-editor-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <div>
      <span class="eyebrow">DIRECTORIO DEL EXPEDIENTE</span>
      <h2 id="participant-editor-title">
        {original ? 'Editar participante' : 'Agregar participante'}
      </h2>
    </div>
    <button class="icon-button" disabled={busy} aria-label="Cerrar participante" onclick={close}
      ><Icon name="close" /></button
    >
  </div>
  <p>Registrar a esta persona no le da acceso al sistema.</p>
  <form class="stack" onsubmit={submit}>
    <ParticipantFields bind:this={fields} bind:draft disabled={fieldsBlocked} />
    <p class="hint">
      El rol y la situaci&#243;n se registran manualmente; no validan identidad ni cambian permisos.
    </p>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if restoreBlocked}<p class="notice" role="status">
        Confirma el expediente y la ficha actuales antes de recuperar el borrador.
      </p>
      <button type="button" class="secondary" disabled={busy} onclick={() => open(original)}
        >Volver a consultar el contexto</button
      >{/if}
    {#if unconfirmed}<p class="notice" role="status">
        No se pudo confirmar el registro anterior. Consulta los datos actuales antes de decidir si
        volver a enviar; registrar otra ficha puede duplicar la anterior.
      </p>{/if}
    {#if conflict || unconfirmed || needsReview || restoredClosed}<button
        type="button"
        class="secondary"
        disabled={busy || restoreBlocked || disabled}
        onclick={refresh}
        >{original ? 'Consultar datos actuales' : 'Consultar directorio actual'}</button
      >{/if}
    {#if candidate}<section class="participant-comparison" aria-label="Valores actuales guardados">
        <h3>Valores actuales guardados</h3>
        <p class="hint">Revisi&#243;n {candidate.revision}</p>
        <ParticipantSummary record={candidate} />
        <p class="hint">
          Guardar mis cambios reemplazar&#225; todos estos valores, incluido el estado, por tu
          formulario ({draft.directory_status === 'active' ? 'Activo' : 'Archivado'}).
        </p>
        {#if needsReview && !typedCurrent}<label
            ><input
              type="checkbox"
              bind:checked={reviewAccepted}
              disabled={fieldsBlocked || disabled}
            />He comparado los valores actuales y decido guardar mi formulario, incluido el estado.</label
          >{/if}
      </section>{/if}
    {#if directoryReviewed}<section aria-label="Directorio actual consultado">
        <ul>
          {#each directory as row}<li>
              {row.name} / {row.status === 'active' ? 'Activo' : 'Archivado'}
            </li>{/each}
        </ul>
        {#if !directory.length}<p>No hay fichas en el directorio consultado.</p>{/if}
        <label
          ><input
            type="checkbox"
            bind:checked={reviewAccepted}
            disabled={fieldsBlocked || disabled}
          />
          He revisado el directorio completo y decido registrar otra ficha; puede duplicar la anterior.</label
        >
      </section>{/if}
    {#if restoredClosed}<p class="notice" role="status">
        El expediente est&#225; cerrado. Puedes consultar el borrador, pero no guardarlo.
      </p>{/if}
    {#if typedCurrent}<p class="notice" role="status">
        Esta ficha ya tiene un perfil tipificado. Puedes consultar tu borrador manual, pero no
        reemplazar el perfil con estos campos.
      </p>{/if}
    <CaseClosedNotice />
    <div class="dialog-actions">
      <button type="button" class="secondary" disabled={busy} onclick={close}>Cancelar</button>
      <button class="primary" disabled={submitBlocked}
        >{submitting
          ? 'Guardando...'
          : candidate
            ? 'Guardar mis cambios'
            : 'Guardar participante'}</button
      >
    </div>
  </form>
</dialog>
