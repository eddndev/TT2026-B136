<script>
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import { participantFailure } from '../lib/participants.mjs';
  export let api;
  export let current;
  export let onconfirmed;
  export let onobserved;
  export let ondenied;
  let dialog;
  let candidate;
  let intended = 'archived';
  let conflict = false;
  let refreshed = false;
  let exhausted = false;
  let busy = false;
  let error = '';
  let blockedByCase = false;
  $: if (blockedByCase && !$administration.closed) {
    error = '';
    blockedByCase = false;
  }
  let alive = true;
  $: already = refreshed && candidate?.directory_status === intended;
  export function open() {
    if ($administration.closed) return;
    candidate = current;
    intended = current.directory_status === 'active' ? 'archived' : 'active';
    conflict = false;
    refreshed = false;
    exhausted = false;
    error = '';
    dialog.showModal();
  }
  function close() {
    if (!busy) dialog.close();
  }
  async function refresh() {
    busy = true;
    try {
      const result = await api.get(current.id);
      if (!alive) return;
      candidate = result;
      error = '';
      refreshed = true;
      onobserved(result);
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
  async function confirm() {
    if (busy || exhausted || already || $administration.closed || (conflict && !refreshed)) return;
    busy = true;
    error = '';
    try {
      const record = await api.changeStatus(current.id, candidate.revision, intended);
      if (!alive) return;
      busy = false;
      close();
      onconfirmed(record);
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (!alive) return;
      conflict = failure.code === 'participant_revision_conflict';
      exhausted = failure.code === 'participant_revision_exhausted';
      refreshed = false;
      error = conflict
        ? 'El participante cambi\u00f3. Consulta sus datos actuales antes de confirmar de nuevo.'
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
  aria-labelledby="participant-status-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <h2 id="participant-status-title">
      {intended === 'archived' ? 'Archivar participante' : 'Reactivar participante'}
    </h2>
    <button class="icon-button" disabled={busy} aria-label="Cerrar cambio de estado" onclick={close}
      ><Icon name="close" /></button
    >
  </div>
  <p>
    Este cambio organiza el directorio y conserva su historial. La situaci&#243;n jur&#237;dica y
    los permisos no cambian.
  </p>
  {#if candidate}<ParticipantSummary record={candidate} />
    <p class="hint">Revisi&#243;n {candidate.revision}</p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if conflict}<button class="secondary" disabled={busy} onclick={refresh}
      >Consultar datos actuales</button
    >{/if}
  {#if refreshed && !already}<p class="notice" role="status">
      Datos actuales consultados. Revisa y confirma el cambio de estado.
    </p>{/if}
  {#if already}<p class="notice" role="status">
      El participante ya est&#225; {intended === 'archived' ? 'archivado' : 'activo'}. No es
      necesario volver a enviar.
    </p>{/if}
  <CaseClosedNotice />
  <div class="dialog-actions">
    <button class="secondary" disabled={busy} onclick={close}
      >{already ? 'Cerrar' : 'Cancelar'}</button
    >
    <button
      class="primary"
      disabled={busy || exhausted || already || $administration.closed || (conflict && !refreshed)}
      onclick={confirm}
      >{busy
        ? 'Guardando...'
        : intended === 'archived'
          ? 'Confirmar archivo'
          : 'Confirmar reactivaci\u00f3n'}</button
    >
  </div>
</dialog>
