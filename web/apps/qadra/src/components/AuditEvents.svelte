<script>
  import Button from './Button.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { auditOutcomeLabel, auditSourceLabel } from '../lib/audit-event-context.mjs';
  export let api, user;
  const day = (value) => value.toISOString().slice(0, 10);
  const today = new Date();
  const following = (value) => day(new Date(Date.parse(`${value}T00:00:00Z`) + 86400000));
  let from = day(new Date(today.getTime() - 7 * 86400000)),
    until = day(today);
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
          until: `${following(until)}T00:00:00Z`,
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
  <section class="card audit-events" aria-label="Movimientos registrados" aria-busy={busy}>
    <div class="section-heading">
      <h2>Movimientos registrados</h2>
    </div>
    <form class="audit-filters" onsubmit={submit}>
      <label>Desde<input type="date" required bind:value={from} oninput={clear} /></label>
      <label>Hasta<input type="date" required bind:value={until} oninput={clear} /></label>
      <label
        >Actor<input
          autocomplete="off"
          bind:value={actor}
          oninput={clear}
          placeholder="Correo o ID de cuenta"
        /></label
      >
      <label>Operaci&#243;n<input autocomplete="off" bind:value={action} oninput={clear} /></label>
      <label>Recurso<input autocomplete="off" bind:value={resource} oninput={clear} /></label>
      <label
        >Por p&#225;gina<select bind:value={limit} onchange={clear}>
          <option value={20}>20</option><option value={50}>50</option><option value={100}
            >100</option
          >
        </select></label
      >
      <div class="action-row">
        <Button variant="primary" disabled={busy}>Consultar</Button>
      </div>
    </form>
    <p class="hint">
      Fechas en UTC, hasta 366 d&#237;as. Los filtros de texto buscan el valor exacto.
    </p>
    {#if busy}<p role="status">Consultando movimientos...</p>
    {:else if error}<p class="notice error" role="alert">{error}</p>
    {:else if value}
      <p class="hint">P&#225;gina {pageNumber} &#183; {value.events.length} movimientos</p>
      {#if value.events.length}
        <ol class="audit-records" aria-label="Movimientos de esta p&#225;gina">
          {#each value.events as event (event.sequence)}
            <li class="audit-record" data-sequence={event.sequence}>
              <time datetime={event.timestamp}>{event.timestamp}</time>
              <dl>
                <div>
                  <dt>Actor</dt>
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
                <div>
                  <dt>Cuenta</dt>
                  <dd>{event.context?.user_id ?? 'No disponible'}</dd>
                </div>
                <div>
                  <dt>IP</dt>
                  <dd>{event.context?.ip_address ?? 'No disponible'}</dd>
                </div>
                <div>
                  <dt>Origen</dt>
                  <dd>{auditSourceLabel(event.context)}</dd>
                </div>
                <div>
                  <dt>Resultado</dt>
                  <dd>{auditOutcomeLabel(event)}</dd>
                </div>
              </dl>
            </li>
          {/each}
        </ol>
      {:else}<p class="notice" role="status">
          Ning&#250;n movimiento coincide con estos filtros.
        </p>{/if}
      <div class="action-row">
        <Button variant="secondary" onclick={() => load()}>Actualizar</Button>
        {#if value.has_more}<Button variant="secondary" onclick={() => load(true)}
            >Siguiente p&#225;gina</Button
          >
        {:else}<span class="hint">No hay m&#225;s movimientos.</span>{/if}
      </div>
    {:else}<p class="hint">Elige un periodo y pulsa Consultar.</p>{/if}
  </section>
{/if}
