<script>
  import { onMount, onDestroy } from 'svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  import { profileLabel } from '../lib/typed-participant-fields.mjs';
  export let api,
    kind,
    ondenied,
    disabled = false,
    busy = false;
  const filterKind = kind;
  let rows = [],
    current = null,
    more = false,
    cursor,
    error = '',
    alive = true;
  async function work(operation) {
    if (disabled || busy) return;
    busy = true;
    error = '';
    try {
      await operation();
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function load(afterId) {
    return work(async () => {
      const result = await api.list({
        status: 'all',
        profile: 'all',
        kind: filterKind || undefined,
        limit: 20,
        afterId,
      });
      if (!alive) return;
      rows = afterId ? [...rows, ...result.participants] : result.participants;
      more = result.has_more;
      cursor = result.next_after_id;
    });
  }
  function open(record) {
    return work(async () => {
      const result = await api.get(record.id);
      if (alive) current = result;
    });
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    busy = false;
    rows = [];
    current = null;
  });
</script>

<section
  class="participant-comparison"
  aria-label="Consulta del directorio completo"
  aria-busy={busy}
>
  <h3>Consulta del directorio completo</h3>
  <p class="hint">
    Incluye fichas activas y archivadas. Tipo consultado: {profileLabel(filterKind) || 'Todos'}. Tu
    formulario se conserva.
  </p>
  <div class="participant-list">
    {#each rows as row}<button
        class="participant-row"
        disabled={disabled || busy}
        onclick={() => open(row)}
        >{row.display_name} / {profileLabel(row.kind || row.procedural_role)} / {row.directory_status ===
        'active'
          ? 'Activo'
          : 'Archivado'}</button
      >{/each}
  </div>
  {#if more}<button class="secondary" disabled={disabled || busy} onclick={() => load(cursor)}
      >Consultar m&#225;s participantes</button
    >{/if}
  {#if current}<ParticipantSummary record={current} />{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</section>
