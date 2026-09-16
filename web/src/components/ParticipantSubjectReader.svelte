<script>
  import { onDestroy } from 'svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import ParticipantSubjectEditor from './ParticipantSubjectEditor.svelte';
  import { canParticipants } from '../lib/participants.mjs';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  export let docs, caseId, user;
  let editor,
    editorBusy = false,
    reading = false;
  $: busy = reading || editorBusy;
  export let api,
    record,
    ondenied,
    disabled = false,
    busy = false;
  let current = null,
    rows = [],
    more = false,
    cursor,
    showHistory = false,
    error = '',
    alive = true;
  async function work(operation) {
    if (busy || disabled) return;
    reading = true;
    error = '';
    try {
      await operation();
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) {
          current = null;
          rows = [];
          ondenied(failure);
        }
      }
    } finally {
      if (alive) reading = false;
    }
  }
  function load() {
    return work(async () => {
      const result = await api.subject(record.id);
      if (alive) current = result;
    });
  }
  function history(beforeRevision) {
    return work(async () => {
      const result = await api.subjectHistory(record.id, { limit: 20, beforeRevision });
      if (!alive) return;
      rows = beforeRevision ? [...rows, ...result.revisions] : result.revisions;
      more = result.has_more;
      cursor = result.next_before_revision;
      showHistory = true;
    });
  }
  onDestroy(() => {
    alive = false;
    busy = false;
    current = null;
    rows = [];
  });
</script>

<div class="participant-history">
  <button class="text-button" disabled={disabled || busy} onclick={load}
    >Consultar identidad actual</button
  >
  {#if current}<section class="participant-comparison" aria-label="Identidad actual consultada">
      <h3>Identidad actual consultada</h3>
      <ParticipantSubjectSummary record={current} />
      {#if canParticipants(user.role, 'manage')}<button
          class="secondary"
          disabled={disabled || busy || $administration.closed}
          onclick={() => editor.open(current)}>Editar identidad</button
        >{/if}
      <p class="hint">La ficha sigue vinculada a la revisi&#243;n {record.revision}.</p>
    </section>{/if}
  <button
    class="text-button"
    disabled={disabled || busy}
    onclick={() => (showHistory ? (showHistory = false) : history())}
    >{showHistory ? 'Ocultar historial de identidad' : 'Consultar historial de identidad'}</button
  >
  {#if showHistory}<section aria-label="Historial de identidad">
      {#each rows as row}<details class="participant-revision">
          <summary>Revisi&#243;n {row.revision} / {row.changed_by.email}</summary
          ><ParticipantSubjectSummary record={row} /><time datetime={row.changed_at}
            >{new Date(row.changed_at).toLocaleString('es-MX')}</time
          >
        </details>{/each}{#if more}<button
          class="secondary"
          disabled={disabled || busy}
          onclick={() => history(cursor)}>Cargar revisiones de identidad anteriores</button
        >{/if}
    </section>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</div>

{#if canParticipants(user.role, 'manage')}<ParticipantSubjectEditor
    bind:this={editor}
    {api}
    {docs}
    {caseId}
    {ondenied}
    bind:pending={editorBusy}
    onconfirmed={(result) => {
      current = result;
    }}
  />{/if}
