<script>
  import { onMount, onDestroy } from 'svelte';
  import DocumentIntegrityIncidentCard from './DocumentIntegrityIncidentCard.svelte';
  import { basicCase } from '../lib/case-administration.mjs';
  import { factSame } from '../lib/procedural-fact-primitives.mjs';
  export let api,
    onopen,
    onknown = () => {};
  const scoped = api.integrityIncidents();
  let rows = [],
    next = null,
    more = false,
    loaded = false,
    busy = false;
  let opening = false,
    denied = false,
    error = '',
    alive = true,
    generation = 0;
  function deny(failure) {
    generation++;
    rows = [];
    next = null;
    more = false;
    busy = false;
    opening = false;
    denied = true;
    error = failure.message;
  }
  async function load(append = false) {
    if (denied || busy || opening || (append && !more)) return;
    const request = ++generation;
    busy = true;
    error = '';
    if (!append) {
      rows = [];
      next = null;
      more = false;
      loaded = false;
    }
    try {
      const result = await scoped.list({ limit: 50, ...(append ? { after_id: next } : {}) });
      if (!alive || request !== generation) return;
      rows = append ? [...rows, ...result.incidents] : result.incidents;
      next = result.next_after_id;
      more = result.has_more;
      loaded = true;
      onknown(rows.length > 0);
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if (failure.status === 403) deny(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function open(incident) {
    if (denied || busy || opening) return;
    const request = ++generation;
    opening = true;
    error = '';
    let administration;
    try {
      const exact = await scoped.get(incident.id);
      if (!alive || request !== generation) return;
      if (!factSame(exact, incident))
        throw new Error('El incidente no conserva la captura consultada.');
      administration = api.caseAdministration(exact.case_id);
      const record = await administration.get();
      if (!alive || request !== generation) return;
      if (record.id !== exact.case_id)
        throw new Error('El expediente no corresponde al incidente.');
      onopen(basicCase(record), {
        case_id: exact.case_id,
        id: exact.document_id,
        version: exact.document_version,
      });
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if (failure.status === 403) deny(failure);
        else if (failure.status === 404) rows = rows.filter((row) => row.id !== incident.id);
      }
    } finally {
      administration?.dispose();
      if (alive && request === generation) opening = false;
    }
  }
  onMount(() => {
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
  });
</script>

<section aria-label="Incidentes de integridad" aria-busy={busy || opening}>
  <div class="page-heading">
    <div>
      <span class="eyebrow">ADMINISTRACI&#211;N DEL DESPACHO</span>
      <h1>Incidentes de integridad</h1>
      <p>Observaciones persistentes de validaci&#243;n documental fallida.</p>
    </div>
    <button class="secondary" disabled={denied || busy || opening} onclick={() => load()}
      >Actualizar incidentes</button
    >
  </div>
  <p class="notice">
    Cada observaci&#243;n registra una validaci&#243;n fallida. No determina su causa ni atribuye
    responsabilidad a una persona.
  </p>
  <p class="hint">Orden por identificador; las fechas describen cada observaci&#243;n.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if busy}<p role="status">Consultando incidentes...</p>{/if}
  {#if opening}<p role="status">Consultando el expediente y la referencia exacta...</p>{/if}
  {#each rows as incident (incident.id)}
    <DocumentIntegrityIncidentCard {incident} onopen={open} disabled={denied || busy || opening} />
  {/each}
  {#if loaded && !busy && !error && !rows.length}<p class="card">
      No hay incidentes registrados.
    </p>{/if}
  {#if more}<button
      class="secondary"
      disabled={denied || busy || opening}
      onclick={() => load(true)}>Cargar m&#225;s incidentes</button
    >{/if}
</section>
