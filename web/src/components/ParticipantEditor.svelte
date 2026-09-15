<script>
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import ParticipantFields from './ParticipantFields.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import { participantDraft, participantFailure } from '../lib/participants.mjs';
  export let api;
  export let onconfirmed;
  export let onobserved;
  export let ondenied;
  let dialog;
  let fields;
  let original = null;
  let draft = participantDraft();
  let candidate = null;
  let busy = false;
  let conflict = false;
  let exhausted = false;
  let error = '';
  let blockedByCase = false;
  $: if (blockedByCase && !$administration.closed) {
    error = '';
    blockedByCase = false;
  }
  let alive = true;
  export function open(record = null) {
    if ($administration.closed) return;
    original = record;
    draft = participantDraft(record || {});
    candidate = null;
    conflict = false;
    exhausted = false;
    error = '';
    fields?.reset();
    dialog.showModal();
  }
  function close() {
    if (busy) return;
    dialog.close();
    draft = participantDraft();
    fields?.reset();
  }
  async function refresh() {
    busy = true;
    try {
      const result = await api.get(original.id);
      if (!alive) return;
      candidate = result;
      error = '';
      await onobserved(result);
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || exhausted || $administration.closed || (conflict && !candidate)) return;
    let values;
    try {
      values = fields.values();
    } catch {
      error = '';
      return;
    }
    busy = true;
    error = '';
    try {
      const record = original
        ? await api.replace(original.id, candidate?.revision ?? original.revision, {
            ...values,
            directory_status: draft.directory_status,
          })
        : await api.create(values);
      if (!alive) return;
      await onconfirmed(record);
      if (!alive) return;
      busy = false;
      close();
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (!alive) return;
      conflict = failure.code === 'participant_revision_conflict';
      exhausted = failure.code === 'participant_revision_exhausted';
      candidate = null;
      error = conflict
        ? 'Los datos cambiaron mientras editabas. Consulta los valores actuales y revisa tu formulario.'
        : participantFailure(failure);
      if ([403, 404].includes(failure.status)) ondenied(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
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
    <ParticipantFields bind:this={fields} bind:draft disabled={busy} />
    <p class="hint">
      El rol y la situaci&#243;n se registran manualmente; no validan identidad ni cambian permisos.
    </p>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if conflict}<button type="button" class="secondary" disabled={busy} onclick={refresh}
        >Consultar datos actuales</button
      >{/if}
    {#if candidate}<section class="participant-comparison" aria-label="Valores actuales guardados">
        <h3>Valores actuales guardados</h3>
        <p class="hint">Revisi&#243;n {candidate.revision}</p>
        <ParticipantSummary record={candidate} />
        <p class="hint">
          Guardar mis cambios reemplazar&#225; todos estos valores, incluido el estado, por tu
          formulario ({draft.directory_status === 'active' ? 'Activo' : 'Archivado'}).
        </p>
      </section>{/if}
    <CaseClosedNotice />
    <div class="dialog-actions">
      <button type="button" class="secondary" disabled={busy} onclick={close}>Cancelar</button>
      <button
        class="primary"
        disabled={busy || exhausted || $administration.closed || (conflict && !candidate)}
        >{busy
          ? 'Guardando...'
          : candidate
            ? 'Guardar mis cambios'
            : 'Guardar participante'}</button
      >
    </div>
  </form>
</dialog>
