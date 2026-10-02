<script>
  export let draft,
    lawyers = [],
    pickerBusy = false,
    pickerError = '',
    pickerMore = false,
    busy = false,
    error = '',
    pending = null,
    onrequest,
    onretry,
    onrefresh,
    onmore;
</script>

<section class="card report-request" aria-label="Solicitar informe">
  <div class="section-heading">
    <div>
      <h2>Solicitar informe</h2>
      <p>Una captura, dos formatos.</p>
    </div>
  </div>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      onrequest();
    }}
  >
    <div class="report-fields">
      <label
        >Creaci&#243;n desde (UTC)<input
          type="date"
          bind:value={draft.from}
          required
          disabled={busy}
        /></label
      >
      <label
        >Creaci&#243;n hasta (excluida, UTC)<input
          type="date"
          bind:value={draft.before}
          required
          disabled={busy}
        /></label
      >
      <label
        ><span id="report-status-label">Estado administrativo</span><select
          aria-labelledby="report-status-label"
          bind:value={draft.status}
          disabled={busy}
          ><option value="all">Todos</option><option value="active">Activos</option><option
            value="closed">Cerrados</option
          ></select
        ></label
      >
      <label
        ><span id="report-assignee-label">Litigante asignado</span><select
          aria-labelledby="report-assignee-label"
          bind:value={draft.assigned}
          disabled={busy || pickerBusy || !!pickerError}
        >
          <option value="">Todos los permitidos</option>
          {#each lawyers as lawyer (lawyer.user_id)}<option value={lawyer.user_id}
              >{lawyer.email}</option
            >{/each}
        </select></label
      >
    </div>
    <p class="hint">
      El inicio se incluye y el final se excluye, ambos a las 00:00 UTC. M&#225;ximo 366 d&#237;as.
    </p>
    <p class="hint">El selector incluye litigantes permitidos, incluso en expedientes cerrados.</p>
    {#if pickerMore}<button
        type="button"
        class="secondary"
        disabled={busy || pickerBusy}
        onclick={onmore}>Cargar m&#225;s litigantes</button
      >{/if}
    {#if pickerBusy}<p role="status">Consultando litigantes disponibles...</p>{/if}
    {#if pickerError}<p class="notice error" role="alert">{pickerError}</p>
      <button type="button" class="secondary" disabled={busy || pickerBusy} onclick={onrefresh}
        >Actualizar litigantes</button
      >{/if}
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if pending}<div class="notice report-retry">
        <p>
          La respuesta anterior no confirm&#243; el resultado. Reintentar conserva la misma
          solicitud y estos filtros:
        </p>
        <p>
          {pending.filters.created_from.slice(0, 10)} a {pending.filters.created_before.slice(
            0,
            10,
          )} (final excluido).
        </p>
        <button type="button" class="secondary" disabled={busy} onclick={onretry}
          >Reintentar solicitud</button
        >
      </div>{/if}
    <button class="primary" type="submit" disabled={busy || pickerBusy || !!pickerError}
      >{busy ? 'Enviando solicitud...' : 'Generar informe'}</button
    >
  </form>
  <p class="hint report-request-note">
    La preparaci&#243;n contin&#250;a en el servidor. Puedes salir de esta pantalla y consultar el
    resultado al regresar.
  </p>
</section>
