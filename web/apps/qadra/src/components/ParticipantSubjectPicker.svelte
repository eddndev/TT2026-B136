<script>
  import { onMount, onDestroy } from 'svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  export let api,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false,
    draft = null,
    canApply = () => true;
  let rows = [],
    current = null,
    name = '',
    query = '',
    kind = '',
    appliedKind = '',
    more = false,
    cursor,
    error = '',
    alive = true;
  const admitted = () => alive && canApply() === true;
  export function captureDraft() {
    return { name, query, kind, appliedKind };
  }
  async function work(operation) {
    if (busy || disabled || !admitted()) return;
    busy = true;
    error = '';
    try {
      await operation();
    } catch (failure) {
      if (admitted()) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function load(afterId) {
    return work(async () => {
      const page = await api.subjects({
        limit: 20,
        name: query || undefined,
        kind: appliedKind || undefined,
        afterId,
      });
      if (!admitted()) return;
      rows = afterId ? [...rows, ...page.subjects] : page.subjects;
      more = page.has_more;
      cursor = page.next_after_id;
      current = null;
    });
  }
  function select(row) {
    return work(async () => {
      const result = await api.subject(row.id);
      if (admitted()) current = result;
    });
  }
  onMount(() => {
    if (draft !== null) {
      if (['name', 'query', 'kind', 'appliedKind'].some((key) => typeof draft[key] !== 'string')) {
        error = 'No se pudo recuperar la consulta de identidad.';
        return;
      }
      ({ name, query, kind, appliedKind } = draft);
    }
    load();
  });
  onDestroy(() => {
    alive = false;
    busy = false;
  });
</script>

<section class="participant-comparison" aria-label="Elegir identidad existente" aria-busy={busy}>
  <h3>Elegir identidad existente</h3>
  <div class="stack">
    <label>Buscar identidad por nombre<input bind:value={name} disabled={disabled || busy} /></label
    ><label
      >Tipo de identidad a buscar<select bind:value={kind} disabled={disabled || busy}
        ><option value="">Todos</option><option value="natural_person">Persona</option><option
          value="institutional_body">&#211;rgano institucional</option
        ></select
      ></label
    >
  </div>
  <button
    type="button"
    class="secondary"
    disabled={disabled || busy}
    onclick={() => {
      query = name;
      appliedKind = kind;
      load();
    }}>Buscar identidades</button
  >
  <div class="participant-list">
    {#each rows as row}<button
        type="button"
        class="participant-row"
        disabled={disabled || busy}
        onclick={() => select(row)}>Consultar identidad: {row.display_name}</button
      >{/each}
  </div>
  {#if more}<button
      type="button"
      class="secondary"
      disabled={disabled || busy}
      onclick={() => load(cursor)}>Cargar m&#225;s identidades</button
    >{/if}
  {#if current}<ParticipantSubjectSummary record={current} /><button
      type="button"
      class="primary"
      disabled={disabled || busy}
      onclick={() => {
        if (admitted()) onselected(current);
      }}>Usar esta identidad</button
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <button type="button" class="text-button" disabled={disabled || busy} onclick={oncancel}
    >Cerrar selector de identidad</button
  >
</section>
