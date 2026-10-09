<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { reportPeriod, reportTypeLabel } from '../lib/case-reports-presentation.mjs';
  import { createReportRequestController } from '../lib/case-report-request-controller.mjs';
  export let api,
    user,
    onstart,
    onrequested,
    ondenied,
    busy = false,
    pickerBusy = false;
  const session = getContext('session-drafts');
  let typeInput, fromInput, beforeInput, statusInput, assigneeInput;
  const controller = createReportRequestController({
    api,
    user,
    session,
    capture: () => ({
      reportType: typeInput?.value ?? state.draft.reportType,
      from: fromInput?.value ?? state.draft.from,
      before: beforeInput?.value ?? state.draft.before,
      status: statusInput?.value ?? state.draft.status,
      assigned: assigneeInput?.value ?? state.draft.assigned,
    }),
    onstate: (value) => {
      state = value;
      busy = value.busy;
      pickerBusy = value.pickerBusy || value.contextBusy;
    },
    onstart,
    onrequested,
    ondenied,
  });
  let state = controller.state();
  $: activity = state.draft.reportType === 'litigator_activity';
  $: pendingPeriod = state.pending ? reportPeriod(state.pending) : null;
  $: locked = state.busy || state.blocked || state.saved || state.denied || state.contextBusy;
  $: unavailable =
    state.draft.assigned && !state.lawyers.some((row) => row.user_id === state.draft.assigned);
  export function deny(failure) {
    controller.deny(failure);
  }
  onMount(() => controller.initialize());
  onDestroy(() => controller.dispose());
</script>

<section class="card report-request" aria-label="Solicitar informe">
  <div class="section-heading">
    <div>
      <h2>Solicitar informe</h2>
      <p>Una captura, dos formatos.</p>
    </div>
  </div>
  {#if state.saved}
    <div class="notice">
      <p>Hay una solicitud pendiente conservada en esta pestana.</p>
      <button
        type="button"
        class="secondary"
        disabled={pickerBusy || busy}
        onclick={controller.context}>Retomar solicitud de informe</button
      >
    </div>
  {/if}
  {#if state.saved || state.dirty || state.pending || (state.blocked && !state.denied)}
    <button
      type="button"
      class="text-button"
      disabled={busy || state.contextBusy}
      onclick={controller.discard}>Descartar solicitud pendiente</button
    >
  {/if}
  <form
    oninput={controller.edit}
    onchange={controller.edit}
    onsubmit={(event) => {
      event.preventDefault();
      controller.request();
    }}
  >
    <div class="report-fields">
      <label
        ><span id="report-type-label">Tipo de informe</span><select
          aria-labelledby="report-type-label"
          bind:this={typeInput}
          bind:value={state.draft.reportType}
          disabled={locked}
        >
          <option value="case_state">Estado y carga de expedientes</option>
          <option value="litigator_activity">Actividad por litigante</option>
        </select></label
      >
      <label
        >{activity ? 'Actividad desde (UTC)' : 'Creaci\u00f3n desde (UTC)'}<input
          type="date"
          bind:this={fromInput}
          bind:value={state.draft.from}
          required
          disabled={locked}
        /></label
      >
      <label
        >{activity
          ? 'Actividad hasta (excluida, UTC)'
          : 'Creaci\u00f3n hasta (excluida, UTC)'}<input
          type="date"
          bind:this={beforeInput}
          bind:value={state.draft.before}
          required
          disabled={locked}
        /></label
      >
      <label
        ><span id="report-status-label">Estado administrativo</span><select
          aria-labelledby="report-status-label"
          bind:this={statusInput}
          bind:value={state.draft.status}
          disabled={locked}
        >
          <option value="all">Todos</option><option value="active">Activos</option>
          <option value="closed">Cerrados</option>
        </select></label
      >
      <label
        ><span id="report-assignee-label"
          >{activity ? 'Litigante autor' : 'Litigante asignado'}</span
        ><select
          aria-labelledby="report-assignee-label"
          bind:this={assigneeInput}
          bind:value={state.draft.assigned}
          disabled={locked || pickerBusy || !!state.pickerError}
        >
          <option value="">Todos los permitidos</option>
          {#if unavailable}<option value={state.draft.assigned}
              >Litigante pendiente de comprobar</option
            >{/if}
          {#each state.lawyers as lawyer (lawyer.user_id)}
            <option value={lawyer.user_id}>{lawyer.email}</option>
          {/each}
        </select></label
      >
    </div>
    <p class="hint">
      El inicio se incluye y el final se excluye, ambos a las 00:00 UTC. M&#225;ximo 366 d&#237;as.
    </p>
    <p class="hint">El selector incluye litigantes permitidos, incluso en expedientes cerrados.</p>
    {#if activity}<p class="hint">
        PDF y CSV incluyen documentos cargados, actuaciones procesales registradas y plazos marcados
        como atendidos, atribuidos a su autor durante el periodo.
      </p>{/if}
    {#if state.pickerMore}<button
        type="button"
        class="secondary"
        disabled={locked || pickerBusy}
        onclick={() => controller.loadPicker(true)}>Cargar m&#225;s litigantes</button
      >{/if}
    {#if pickerBusy}<p role="status">Consultando litigantes disponibles...</p>{/if}
    {#if state.pickerError}<p class="notice error" role="alert">{state.pickerError}</p>
      <button
        type="button"
        class="secondary"
        disabled={locked || pickerBusy}
        onclick={() => controller.loadPicker()}>Actualizar litigantes</button
      >{/if}
    {#if state.error}<p class="notice error" role="alert">{state.error}</p>{/if}
    {#if state.blocked && !state.denied}
      <button
        type="button"
        class="secondary"
        disabled={busy || pickerBusy}
        onclick={controller.context}>Volver a consultar el contexto</button
      >
    {/if}
    {#if state.pending}<div class="notice report-retry">
        <p>
          La respuesta anterior no confirm&#243; el resultado. Reintentar conserva la misma
          solicitud y estos filtros:
        </p>
        <p>
          {reportTypeLabel(state.pending)}: {pendingPeriod.from.slice(0, 10)} a
          {pendingPeriod.before.slice(0, 10)} (final excluido).
        </p>
        <button
          type="button"
          class="secondary"
          disabled={locked || pickerBusy || !!state.pickerError}
          onclick={() => controller.request(true)}>Reintentar solicitud</button
        >
      </div>{/if}
    <button class="primary" type="submit" disabled={locked || pickerBusy || !!state.pickerError}
      >{busy ? 'Enviando solicitud...' : 'Generar informe'}</button
    >
  </form>
  <p class="hint report-request-note">
    La preparaci&#243;n contin&#250;a en el servidor. Puedes salir de esta pantalla y consultar el
    resultado al regresar.
  </p>
</section>
