<script>
  import { onMount, onDestroy } from 'svelte';
  import FactResolutionPicker from './FactResolutionPicker.svelte';
  import FactResultPicker from './FactResultPicker.svelte';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { factDeclarationLabel, classLabels } from './fact-field-labels.mjs';
  import { deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  export let api,
    caseId,
    family,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  const scoped = family === 'hearing_result' ? null : api.caseResolutions(caseId);
  let notificationsApi = null,
    resolutions = [],
    notifications = [],
    revisions = [],
    resolutionId = null,
    notificationId = null,
    exact = null,
    alive = true,
    error = '',
    working = false,
    childBusy = false,
    more = false,
    notificationMore = false,
    historyMore = false,
    cursor,
    notificationCursor,
    historyCursor;
  $: busy = working || childBusy;
  async function work(task) {
    if (busy || disabled) return;
    working = true;
    error = '';
    try {
      const row = await task();
      if (alive) return row;
    } catch (failure) {
      if (alive) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) working = false;
    }
  }
  async function list(afterId) {
    resolutionId = null;
    notifications = [];
    revisions = [];
    exact = null;
    const page = await work(() => scoped.list({ status: 'all', limit: 20, afterId }));
    if (page) {
      resolutions = page.resolutions;
      more = page.has_more;
      cursor = page.next_after_id;
    }
  }
  async function chooseResolution(id, afterId) {
    if (busy || disabled) return;
    if (resolutionId !== id) {
      notificationsApi?.dispose();
      notificationsApi = api.caseNotifications(caseId, id);
    }
    resolutionId = id;
    revisions = [];
    exact = null;
    notificationId = null;
    if (family === 'resolution') return;
    const page = await work(() => notificationsApi.list({ status: 'all', limit: 20, afterId }));
    if (page) {
      notifications = page.notifications;
      notificationMore = page.has_more;
      notificationCursor = page.next_after_id;
    }
  }
  async function history(id, beforeRevision) {
    exact = null;
    const page = await work(() => notificationsApi.history(id, { limit: 10, beforeRevision }));
    if (page) {
      notificationId = id;
      revisions = page.revisions;
      historyMore = page.has_more;
      historyCursor = page.next_before_revision;
    }
  }
  async function read(revision) {
    exact = null;
    const row = await work(() => notificationsApi.revision(notificationId, revision));
    if (row) exact = row;
  }
  onMount(() => {
    if (scoped) list();
  });
  onDestroy(() => {
    alive = false;
    busy = false;
    scoped?.dispose();
    notificationsApi?.dispose();
  });
</script>

<section class="case-comparison" aria-label="Elegir fuente del plazo" aria-busy={busy}>
  {#if family === 'hearing_result'}<FactResultPicker
      {api}
      {caseId}
      {ondenied}
      {disabled}
      bind:busy={childBusy}
      {oncancel}
      onselected={({ reference, record }) =>
        onselected({ reference: { family, ...reference }, record })}
    />
  {:else}
    <h4>Resoluciones del expediente</h4>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#each resolutions as row}<div class="hearing-result-picker-row">
        <p>{factDeclarationLabel(row.class, classLabels)} / {factTimeLabel(row.issued_at)}</p>
        <p>
          Revisi&#243;n {row.revision} / {row.status === 'withdrawn' ? 'Retirada' : 'Registrada'}
        </p>
        <button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => chooseResolution(row.id)}
          aria-label={`Consultar ${family === 'resolution' ? 'revisiones' : 'notificaciones'} de resolucion ${row.id}`}
        >
          {family === 'resolution'
            ? 'Consultar revisiones de la fuente'
            : 'Consultar notificaciones de la fuente'}</button
        >
      </div>{/each}
    {#if !resolutions.length && !busy}<p>No hay resoluciones en esta p&#225;gina.</p>{/if}
    <div class="action-row">
      <button type="button" class="secondary" disabled={disabled || busy} onclick={() => list()}
        >Primera p&#225;gina de resoluciones</button
      >
      {#if more}<button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => list(cursor)}>Siguientes resoluciones</button
        >{/if}
    </div>
    {#if resolutionId && family === 'resolution'}{#key resolutionId}<FactResolutionPicker
          {api}
          {caseId}
          {resolutionId}
          {ondenied}
          disabled={disabled || working}
          bind:busy={childBusy}
          oncancel={() => (resolutionId = null)}
          onselected={(record) =>
            onselected({ reference: { family, id: record.id, revision: record.revision }, record })}
        />{/key}{/if}
    {#if resolutionId && family === 'notification'}<h5>Notificaciones de la resoluci&#243;n</h5>
      {#each notifications as row}<div class="hearing-result-picker-row">
          <p>{factTimeLabel(row.practiced_at)} / Revisi&#243;n {row.revision}</p>
          <button
            type="button"
            class="secondary"
            disabled={disabled || busy}
            onclick={() => history(row.id)}>Revisiones de notificaci&#243;n {row.id}</button
          >
        </div>{/each}
      {#if !notifications.length && !busy}<p>No hay notificaciones en esta p&#225;gina.</p>{/if}
      {#if notificationMore}<button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => chooseResolution(resolutionId, notificationCursor)}
          >Siguientes notificaciones</button
        >{/if}
      {#each revisions as row}<button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => read(row.revision)}
          >Consultar notificaci&#243;n revisi&#243;n {row.revision}</button
        >{/each}
      {#if historyMore}<button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => history(notificationId, historyCursor)}
          >Revisiones anteriores de notificaci&#243;n</button
        >{/if}
      {#if exact}<p>
          Notificaci&#243;n revisi&#243;n {exact.revision} / Resoluci&#243;n vinculada revisi&#243;n {exact
            .values.resolution.revision}
        </p>
        <p>{factTimeLabel(exact.values.practiced_at)}</p>
        <p class="case-multiline">{exact.values.summary}</p>
        <button
          type="button"
          class="primary"
          disabled={disabled || busy}
          onclick={() =>
            onselected({
              reference: {
                family,
                id: exact.id,
                revision: exact.revision,
                resolution: { ...exact.values.resolution },
              },
              record: exact,
            })}>Vincular esta notificaci&#243;n exacta</button
        >
      {/if}
    {/if}
    <button type="button" class="text-button" disabled={busy} onclick={oncancel}
      >Cerrar selector de fuente</button
    >
  {/if}
</section>
