<script>
  import { onMount, onDestroy } from 'svelte';
  import CaseValues from './CaseValues.svelte';
  export let api, ondenied;
  let revisions = [],
    hasMore = false,
    next,
    busy = false,
    error = '',
    alive = true,
    generation = 0;
  async function load(more = false) {
    const current = ++generation;
    busy = true;
    error = '';
    try {
      const page = await api.history({ beforeRevision: more ? next : undefined });
      if (!alive || current !== generation) return;
      revisions = more ? [...revisions, ...page.revisions] : page.revisions;
      hasMore = page.has_more;
      next = page.next_before_revision;
    } catch (failure) {
      if (alive && current === generation) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) {
          revisions = [];
          ondenied(failure);
        }
      }
    } finally {
      if (alive && current === generation) busy = false;
    }
  }
  onMount(() => {
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<section class="case-history" aria-label="Historial administrativo" aria-busy={busy}>
  <div class="section-heading">
    <h3>Historial administrativo</h3>
    <button class="secondary" disabled={busy} onclick={() => load()}
      >Actualizar historial administrativo</button
    >
  </div>
  {#each revisions as record}<details class="participant-revision">
      <summary
        ><span
          ><strong>Revisi&#243;n {record.revision}</strong><span>{record.changed_by.email}</span>
          <time datetime={record.changed_at} title={record.changed_at}
            >{new Date(record.changed_at).toLocaleString('es-MX')}</time
          ></span
        ></summary
      >
      <CaseValues {record} />
      <p class="hint">Autor: {record.changed_by.email} / {record.changed_by.id}</p>
      <p class="hint">Huella de los valores: <code>{record.values_digest}</code></p>
    </details>{/each}
  {#if !busy && !revisions.length && !error}<p class="hint">
      Todav&#237;a no hay revisiones administrativas registradas.
    </p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if hasMore}<button class="secondary" disabled={busy} onclick={() => load(true)}
      >Cargar revisiones anteriores</button
    >{/if}
</section>
