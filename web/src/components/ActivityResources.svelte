<script>
  import { onMount, onDestroy } from 'svelte';
  import { resourceActivityFailure, resourceDenied } from '../lib/resource-activity-errors.mjs';
  import { resourceActivityStatuses } from '../lib/resource-activity-values.mjs';
  import { resourceKinds } from '../lib/procedural-resource-values.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  export let api,
    caseId,
    kind,
    record,
    ondenied,
    onopen,
    disabled = false;
  const scoped = api.caseActivityResources(caseId, { kind, id: record.id });
  let rows = [],
    checkedAt = null,
    error = '',
    busy = false,
    alive = true,
    generation = 0;
  let status = 'linked',
    applied = 'linked',
    cursors = [undefined],
    index = 0,
    more = false,
    next;
  async function load(position = 0) {
    const request = ++generation;
    busy = true;
    error = '';
    rows = [];
    checkedAt = null;
    more = false;
    index = position;
    try {
      const page = await scoped.list({ status: applied, afterId: cursors[position] });
      if (!alive || request !== generation) return;
      rows = page.associations;
      checkedAt = page.checked_at;
      more = page.has_more;
      next = page.next_after_id;
    } catch (failure) {
      if (alive && request === generation) {
        error = resourceActivityFailure(failure);
        if (resourceDenied(failure)) {
          rows = [];
          checkedAt = null;
          ondenied(failure);
        }
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function open(row) {
    if (!alive || busy || disabled) return;
    const association = row.association;
    onopen({
      case_id: caseId,
      resource: structuredClone(association.selection.resource),
      association: {
        id: association.id,
        revision: association.revision,
        capture_digest: association.receipt.capture_digest,
      },
      origin: {
        kind,
        case_id: caseId,
        revision: record.revision,
        [kind === 'hearing' ? 'hearing_id' : 'deadline_id']: record.id,
      },
    });
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

<section class="card fact-detail" aria-label="Recursos relacionados" aria-busy={busy}>
  <h2>Recursos relacionados</h2>
  <p>Estado actual de los v&#237;nculos organizativos con esta actividad.</p>
  <p class="hint">
    Esta consulta no representa el estado al momento de emisi&#243;n de una alerta. Cada
    v&#237;nculo conserva sus propias revisiones hist&#243;ricas.
  </p>
  <p>Revisi&#243;n consultada de la actividad: {record.revision}</p>
  {#if checkedAt}<p class="hint">Consultado el {deadlineInstantLabel(checkedAt)}</p>{/if}
  <div class="action-row">
    <label
      >Estado del v&#237;nculo<select bind:value={status} disabled={busy || disabled}>
        <option value="linked">Vinculados</option><option value="unlinked">Desvinculados</option>
        <option value="all">Todos</option>
      </select></label
    >
    <button
      class="secondary"
      disabled={busy || disabled}
      onclick={() => {
        applied = status;
        cursors = [undefined];
        load();
      }}>Consultar recursos relacionados</button
    >
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if busy}<p role="status">Consultando recursos relacionados...</p>{/if}
  {#each rows as row (row.association.id)}
    {@const association = row.association}
    <article class="case-comparison" data-association-id={association.id}>
      <div class="section-heading">
        <h3>{association.sources.resource.values.title}</h3>
        <span class="badge" class:info={association.status === 'linked'}
          >{resourceActivityStatuses[association.status]}</span
        >
      </div>
      <p>
        {resourceKinds[association.sources.resource.values.kind]} / Recurso revisi&#243;n {association
          .selection.resource.revision}
      </p>
      <p>Revisi&#243;n vinculada de la actividad: {association.selection.target.revision}</p>
      <p class="hint">
        Actividad actual al consultar: revisi&#243;n {row.current_target.record.revision}.
        V&#237;nculo revisi&#243;n {association.revision}.
      </p>
      {#if association.selection.act}<p>
          Acto del recurso: revisi&#243;n {association.selection.act.revision}
        </p>{/if}
      {#if association.reason}<p class="case-multiline">Motivo: {association.reason}</p>{/if}
      <button
        class="secondary"
        disabled={busy || disabled}
        aria-label={`Abrir recurso vinculado ${association.id}`}
        onclick={() => open(row)}>Abrir recurso y v&#237;nculo exactos</button
      >
    </article>
  {/each}
  {#if !rows.length && !busy && !error}<p>No hay recursos relacionados en esta consulta.</p>{/if}
  <div class="action-row">
    <button class="secondary" disabled={busy || disabled || !index} onclick={() => load(index - 1)}
      >Recursos relacionados anteriores</button
    >
    <button
      class="secondary"
      disabled={busy || disabled || !more}
      onclick={() => {
        cursors = [...cursors.slice(0, index + 1), next];
        load(index + 1);
      }}>Siguientes recursos relacionados</button
    >
  </div>
</section>
