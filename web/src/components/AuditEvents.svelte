<script>
  import { onMount, onDestroy } from 'svelte';
  import '../styles/audit-events.css';
  export let api, user;
  const day = (value) => value.toISOString().slice(0, 10);
  const today = new Date();
  let from = day(new Date(today.getTime() - 7 * 86400000)),
    until = day(new Date(today.getTime() + 86400000));
  let actor = '',
    action = '',
    resource = '',
    limit = 20;
  let scoped = null,
    value = null,
    submitted = null,
    busy = false,
    error = '';
  let mounted = false,
    alive = true,
    seen = null,
    generation = 0,
    pageNumber = 0;
  $: allowed = user?.role === 'owner';
  $: identity = `${user?.id}:${user?.email}:${user?.role}`;
  $: if (mounted && identity !== seen) open(identity);
  function clear() {
    generation++;
    scoped?.dispose();
    scoped = allowed && alive ? api.auditEvents() : null;
    value = null;
    submitted = null;
    error = '';
    busy = false;
    pageNumber = 0;
  }
  function open(current) {
    seen = current;
    clear();
    actor = '';
    action = '';
    resource = '';
  }
  async function load(next = false) {
    if (!scoped || busy || !alive || !allowed) return;
    const query = next
      ? { ...submitted, cursor: value.next_cursor }
      : {
          from: `${from}T00:00:00Z`,
          until: `${until}T00:00:00Z`,
          limit,
          ...(actor ? { actor } : {}),
          ...(action ? { action } : {}),
          ...(resource ? { resource } : {}),
        };
    const current = ++generation,
      context = identity,
      nextNumber = next ? pageNumber + 1 : 1;
    const client = scoped;
    value = null;
    busy = true;
    error = '';
    try {
      const result = await client.list(query);
      if (!alive || current !== generation || context !== identity) return;
      value = result;
      submitted = { ...query, cursor: null };
      pageNumber = nextNumber;
    } catch (failure) {
      if (!alive || current !== generation || context !== identity) return;
      value = null;
      submitted = null;
      pageNumber = 0;
      error = failure.message;
      scoped?.dispose();
      scoped = allowed ? api.auditEvents() : null;
    } finally {
      if (alive && current === generation && context === identity) busy = false;
    }
  }
  function submit(event) {
    event.preventDefault();
    load();
  }
  onMount(() => {
    mounted = true;
  });
  onDestroy(() => {
    alive = false;
    generation++;
    scoped?.dispose();
    scoped = null;
    value = null;
    submitted = null;
    actor = '';
    action = '';
    resource = '';
    error = '';
  });
</script>

{#if allowed}
  <section class="card audit-events" aria-label="Actividad registrada" aria-busy={busy}>
    <div class="section-heading">
      <div>
        <h2>Actividad registrada</h2>
        <p>Consulta eventos del despacho por fecha, actor, operaci&#243;n o recurso.</p>
      </div>
    </div>
    <form class="audit-filters" onsubmit={submit}>
      <label
        >Desde (UTC, incluido)<input
          type="date"
          required
          bind:value={from}
          oninput={clear}
        /></label
      >
      <label
        >Hasta (UTC, excluido)<input
          type="date"
          required
          bind:value={until}
          oninput={clear}
        /></label
      >
      <label
        >Actor registrado<input
          autocomplete="off"
          bind:value={actor}
          oninput={clear}
          placeholder="Texto exacto, opcional"
        /></label
      >
      <label
        >Operaci&#243;n registrada<input
          autocomplete="off"
          bind:value={action}
          oninput={clear}
          placeholder="Texto exacto, opcional"
        /></label
      >
      <label
        >Recurso registrado<input
          autocomplete="off"
          bind:value={resource}
          oninput={clear}
          placeholder="Texto exacto, opcional"
        /></label
      >
      <label
        >Eventos por p&#225;gina<select bind:value={limit} onchange={clear}>
          <option value={20}>20</option><option value={50}>50</option><option value={100}
            >100</option
          >
        </select></label
      >
      <div class="action-row">
        <button class="primary" disabled={busy}>Consultar actividad</button>
      </div>
    </form>
    <p class="hint">
      El periodo usa d&#237;as UTC y puede abarcar hasta 366 d&#237;as. Los filtros de texto
      distinguen may&#250;sculas y espacios.
    </p>
    {#if busy}<p role="status">Consultando la actividad registrada...</p>
    {:else if error}<p class="notice error" role="alert">{error}</p>
    {:else if value}
      <p class="hint">
        P&#225;gina {pageNumber}: {value.events.length} eventos. Consulta del
        <time datetime={value.checked_at}>{value.checked_at}</time>. Las p&#225;ginas siguientes
        conservan el mismo corte de actividad.
      </p>
      {#if value.events.length}
        <ol class="audit-records" aria-label="Eventos de esta pagina">
          {#each value.events as event (event.sequence)}
            <li class="audit-record" data-sequence={event.sequence}>
              <time datetime={event.timestamp}>{event.timestamp}</time>
              <dl>
                <div>
                  <dt>Actor registrado</dt>
                  <dd>{event.actor}</dd>
                </div>
                <div>
                  <dt>Operaci&#243;n</dt>
                  <dd>{event.action}</dd>
                </div>
                <div>
                  <dt>Recurso</dt>
                  <dd>{event.resource}</dd>
                </div>
                <div>
                  <dt>Secuencia</dt>
                  <dd>{event.sequence}</dd>
                </div>
              </dl>
            </li>
          {/each}
        </ol>
      {:else}<p class="notice" role="status">
          No hay eventos que coincidan con estos filtros.
        </p>{/if}
      <div class="action-row">
        <button class="secondary" onclick={() => load()}>Actualizar actividad</button>
        {#if value.has_more}<button class="secondary" onclick={() => load(true)}
            >Cargar siguiente p&#225;gina</button
          >
        {:else}<span class="hint">Fin de esta consulta.</span>{/if}
      </div>
    {:else}<p class="hint">Elige los filtros y pulsa Consultar actividad.</p>{/if}
    <p class="hint">
      Cada p&#225;gina reemplaza la anterior. Actualizar incorpora los eventos registrados
      despu&#233;s. Para comprobar la integridad de la bit&#225;cora usa Verificar cadena.
    </p>
  </section>
{/if}
