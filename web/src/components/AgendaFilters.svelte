<script>
  import { shiftAgendaPeriod } from '../lib/agenda-periods.mjs';
  export let value,
    onapply,
    disabled = false;
  let previous = value,
    draft = { ...value },
    error = '';
  $: if (value !== previous) {
    previous = value;
    draft = { ...value };
    error = '';
  }
  function navigate(direction) {
    try {
      draft = { ...draft, date: shiftAgendaPeriod(draft.view, draft.date, direction) };
      error = '';
      onapply({ ...draft });
    } catch (failure) {
      error = failure.message;
    }
  }
  function changeKind(event) {
    const kind = event.currentTarget.value;
    draft = {
      ...draft,
      kind,
      hearing_status: kind === 'deadline' ? 'scheduled' : draft.hearing_status,
    };
  }
</script>

<form
  class="hearing-agenda-filters"
  onsubmit={(event) => {
    event.preventDefault();
    error = '';
    onapply({ ...draft });
  }}
>
  <fieldset class="case-offenses" {disabled}>
    <legend>Periodo y actividades</legend>
    <div class="case-field-grid">
      <label
        >Vista de agenda<select bind:value={draft.view}>
          <option value="day">D&#237;a</option><option value="week">Semana</option>
          <option value="month">Mes</option><option value="custom">Rango personalizado</option>
        </select></label
      >
      {#if draft.view === 'custom'}
        <label
          >Desde (incluido)<input
            type="date"
            min="0001-01-01"
            max="9999-12-31"
            required
            bind:value={draft.from}
          /></label
        >
        <label
          >Hasta (excluido)<input
            type="date"
            min="0001-01-01"
            max="9999-12-31"
            required
            bind:value={draft.until}
          /></label
        >
      {:else}
        <label
          >Fecha de referencia<input
            type="date"
            min="0001-01-01"
            max="9999-12-31"
            required
            bind:value={draft.date}
          /></label
        >
      {/if}
      <label
        >Desfase de consulta<input
          required
          maxlength="6"
          bind:value={draft.offset}
          placeholder="+00:00"
        /></label
      >
      <label
        >Tipo de actividad<select value={draft.kind} onchange={changeKind}>
          <option value="all">Audiencias y plazos</option><option value="hearing">Audiencias</option
          >
          <option value="deadline">Plazos</option>
        </select></label
      >
      <label
        >Estado de audiencia<select
          bind:value={draft.hearing_status}
          disabled={draft.kind === 'deadline'}
        >
          <option value="scheduled">Programadas</option><option value="cancelled">Canceladas</option
          >
          <option value="all">Todas</option>
        </select></label
      >
    </div>
    <p class="hint">
      El periodo empieza a las 00:00 en el desfase indicado y excluye el inicio del d&#237;a final.
      Las semanas empiezan el lunes. M&#225;ximo 366 d&#237;as; el estado s&#243;lo filtra
      audiencias.
    </p>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    <div class="action-row agenda-period-actions">
      {#if draft.view !== 'custom'}
        <button type="button" class="secondary" onclick={() => navigate(-1)}
          >Periodo anterior</button
        >
        <button type="button" class="secondary" onclick={() => navigate(1)}
          >Periodo siguiente</button
        >
      {/if}
      <button class="primary" type="submit">Consultar Agenda</button>
    </div>
  </fieldset>
</form>
