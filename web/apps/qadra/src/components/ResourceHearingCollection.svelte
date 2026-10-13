<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import ResourceHearingDetail from './ResourceHearingDetail.svelte';
  import { resourceHearingCreationScope } from '../lib/resource-hearing-creation.mjs';
  import { resourceHearingKinds } from '../lib/resource-hearing-values.mjs';
  import { hearingTimeLabel } from '../lib/hearings.mjs';
  import { basicCase } from '../lib/case-administration.mjs';
  import { resourceDenied } from '../lib/procedural-resource-errors.mjs';

  export let api,
    user,
    caseId,
    resource,
    ondenied,
    disabled = false,
    created = null,
    pending = false;
  const session = getContext('session-drafts');
  const scoped = api.caseResourceHearings(caseId, resource.id);
  const administration = api.caseAdministration(caseId);
  const actor = { id: user.id, email: user.email, role: user.role };
  let rows = [],
    selected = null,
    caseRecord = null,
    next = null,
    more = false,
    busy = false,
    error = '',
    alive = true,
    generation = 0,
    initialized = false,
    seen = null;
  $: pending = busy;
  $: if (initialized && created && created !== seen) {
    seen = created;
    load(false, created);
  }

  function admitted() {
    const current = session?.principal() || user;
    return (
      alive &&
      (!session || session.canAdmit()) &&
      current?.id === actor.id &&
      current?.email === actor.email &&
      current?.role === actor.role &&
      ['owner', 'litigator', 'paralegal'].includes(actor.role)
    );
  }
  const current = (request) => admitted() && request === generation;
  async function authorize(request) {
    const value = await administration.get();
    if (!current(request)) return null;
    if (
      value.id !== caseId ||
      !['active', 'closed'].includes(value.administration?.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente de la audiencia.');
    return basicCase(value);
  }
  function fail(failure) {
    error = failure.message || 'No se pudo consultar la audiencia de recurso.';
    if (resourceDenied(failure)) {
      generation++;
      rows = [];
      selected = caseRecord = null;
      next = null;
      more = false;
      busy = false;
      if (failure.status === 403 || failure.code === 'case_not_found')
        session?.registry.denyContext(caseId);
      ondenied(failure);
    }
  }
  async function load(append = false, confirmation = null) {
    if (!admitted() || (append && (busy || !more))) return;
    const request = ++generation;
    const cursor = append ? next : undefined;
    busy = true;
    error = '';
    selected = caseRecord = null;
    if (!append) {
      rows = [];
      next = null;
      more = false;
    }
    try {
      const confirmed = confirmation
        ? resourceHearingCreationScope(structuredClone(confirmation), caseId, resource.id)
        : null;
      const authorized = await authorize(request);
      if (!authorized) return;
      const page = await scoped.list({ limit: 10, ...(cursor ? { afterId: cursor } : {}) });
      if (!current(request)) return;
      rows = append ? [...rows, ...page.items] : page.items;
      next = page.next_after_id;
      more = page.has_more;
      caseRecord = authorized;
      if (confirmed) selected = confirmed;
    } catch (failure) {
      if (current(request)) fail(failure);
    } finally {
      if (current(request)) busy = false;
    }
  }
  async function open(creation) {
    if (!admitted() || busy || disabled) return;
    const row = structuredClone(creation.hearing),
      request = ++generation;
    selected = null;
    busy = true;
    error = '';
    try {
      const authorized = await authorize(request);
      if (!authorized) return;
      const value = await scoped.exact({
        case_id: row.case_id,
        resource_id: row.resource_id,
        id: row.id,
        revision: row.revision,
        kind: row.values.kind,
        scheduled_at: row.values.scheduled_at,
        modality: row.values.modality,
        participant_count: row.values.participants.length,
        association_id: row.association_id,
        capture_digest: row.capture_digest,
      });
      if (current(request)) {
        selected = value;
        caseRecord = authorized;
      }
    } catch (failure) {
      if (current(request)) fail(failure);
    } finally {
      if (current(request)) busy = false;
    }
  }
  onMount(() => {
    initialized = true;
    if (!created) load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
    administration.dispose();
    pending = false;
  });
</script>

<section class="fact-collection" aria-label="Audiencias del recurso" aria-busy={busy}>
  <div class="section-heading">
    <h3>Audiencias del recurso</h3>
    <button class="secondary" disabled={disabled || busy} onclick={() => load()}
      >Actualizar audiencias del recurso</button
    >
  </div>
  <p class="hint">Programaciones conservadas, incluso si su asociaci&#243;n fue desvinculada.</p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !busy && !error && !rows.length}<p>No hay audiencias propias en esta consulta.</p>{/if}
  <div class="hearing-pick-list">
    {#each rows as value (value.hearing.id)}
      <button
        class="card activity-card"
        disabled={disabled || busy}
        aria-label={`Consultar audiencia de recurso ${value.hearing.id}`}
        onclick={() => open(value)}
      >
        <strong>{resourceHearingKinds[value.hearing.values.kind]}</strong>
        <span>{hearingTimeLabel(value.hearing.values.scheduled_at)}</span>
        <span>{value.hearing.values.venue}</span>
      </button>
    {/each}
  </div>
  {#if more}<button class="secondary" disabled={disabled || busy} onclick={() => load(true)}
      >Cargar mas audiencias del recurso</button
    >{/if}
  {#if selected && caseRecord}
    <ResourceHearingDetail
      value={selected}
      {caseRecord}
      onclose={() => {
        selected = null;
      }}
    />
  {/if}
</section>
