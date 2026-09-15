<script>
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import CaseValues from './CaseValues.svelte';
  import { caseFailure } from '../lib/case-administration.mjs';
  export let api, record, onconfirmed, onobserved, ondenied;
  let uncertain = false;
  let dialog,
    candidate,
    intended = 'closed',
    conflict = false,
    refreshed = false;
  let busy = false,
    exhausted = false,
    error = '',
    alive = true;
  $: already = refreshed && candidate?.administration.administrative_status === intended;
  export function open() {
    candidate = record;
    intended = record.administration.administrative_status === 'active' ? 'closed' : 'active';
    conflict = false;
    uncertain = false;
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
      const result = await api.get();
      if (alive) {
        candidate = result;
        error = '';
        refreshed = true;
        onobserved(result);
      }
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function confirm() {
    if (busy || exhausted || already || ((conflict || uncertain) && !refreshed)) return;
    busy = true;
    error = '';
    try {
      const result = await api.status(candidate.administration.revision, intended);
      if (alive) {
        busy = false;
        close();
        onconfirmed(result);
      }
    } catch (failure) {
      if (alive) {
        uncertain = !failure.status;
        conflict = failure.code === 'case_revision_conflict';
        exhausted = failure.code === 'case_revision_exhausted';
        refreshed = false;
        error = caseFailure(failure);
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
  });
</script>

<dialog
  class="upload-dialog case-status-dialog"
  bind:this={dialog}
  aria-labelledby="case-status-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <h2 id="case-status-title">
      {intended === 'closed' ? 'Cerrar administrativamente' : 'Reactivar expediente'}
    </h2>
    <button
      class="icon-button"
      disabled={busy}
      aria-label="Cerrar cambio administrativo"
      onclick={close}><Icon name="close" /></button
    >
  </div>
  <p>
    {intended === 'closed'
      ? 'El expediente seguir\u00e1 disponible para consulta, verificaci\u00f3n y exportaci\u00f3n. Se suspender\u00e1n las nuevas modificaciones.'
      : 'Se permitir\u00e1n nuevas modificaciones. Los participantes archivados conservar\u00e1n su estado.'}
    La etapa registrada no cambia.
  </p>
  {#if candidate}<CaseValues record={candidate.administration} />
    <p class="hint">Revisi&#243;n {candidate.administration.revision}</p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if conflict || uncertain}<button class="secondary" disabled={busy} onclick={refresh}
      >Consultar datos actuales</button
    >{/if}
  {#if refreshed}<p class="notice" role="status">
      {already
        ? 'El expediente ya tiene el estado solicitado. No es necesario volver a enviar.'
        : 'Datos actuales consultados. Revisa y confirma el cambio de estado.'}
    </p>{/if}
  <div class="dialog-actions">
    <button class="secondary" disabled={busy} onclick={close}
      >{already ? 'Cerrar' : 'Cancelar'}</button
    >
    <button
      class="primary"
      disabled={busy || exhausted || already || ((conflict || uncertain) && !refreshed)}
      onclick={confirm}
      >{busy
        ? 'Guardando...'
        : intended === 'closed'
          ? 'Confirmar cierre administrativo'
          : 'Confirmar reactivaci\u00f3n'}</button
    >
  </div>
</dialog>
