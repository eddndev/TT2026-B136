<script>
  // One row of agenda filters; the calendar beside the list moves the period.
  import Button from './Button.svelte';
  import FilterBar from './FilterBar.svelte';
  import FilterField from './FilterField.svelte';
  import MoreFilters from './MoreFilters.svelte';
  export let value,
    onapply,
    disabled = false;
  let previous = value,
    draft = { ...value };
  $: if (value !== previous) {
    previous = value;
    draft = { ...value };
  }
  function changeKind(event) {
    const kind = event.currentTarget.value;
    draft = {
      ...draft,
      kind,
      hearing_status:
        kind === 'deadline' || (kind === 'resource_hearing' && draft.hearing_status === 'cancelled')
          ? 'scheduled'
          : draft.hearing_status,
    };
  }
</script>

<form
  class="hearing-agenda-filters"
  onsubmit={(event) => {
    event.preventDefault();
    onapply({ ...draft });
  }}
>
  <FilterBar as="fieldset" class="min-w-0" {disabled}>
    <legend class="sr-only">Periodo y actividades</legend>
    <FilterField
      >Vista de agenda<select bind:value={draft.view}>
        <option value="day">D&#237;a</option><option value="week">Semana</option>
        <option value="month">Mes</option><option value="custom">Rango personalizado</option>
      </select></FilterField
    >
    {#if draft.view === 'custom'}
      <FilterField
        >Desde (incluido)<input
          type="date"
          min="0001-01-01"
          max="9999-12-31"
          required
          bind:value={draft.from}
        /></FilterField
      >
      <FilterField
        >Hasta (excluido)<input
          type="date"
          min="0001-01-01"
          max="9999-12-31"
          required
          bind:value={draft.until}
        /></FilterField
      >
    {:else}
      <FilterField
        >Fecha de referencia<input
          type="date"
          min="0001-01-01"
          max="9999-12-31"
          required
          bind:value={draft.date}
        /></FilterField
      >
    {/if}
    <FilterField size="narrow"
      >Desfase de consulta<input
        required
        maxlength="6"
        bind:value={draft.offset}
        placeholder="+00:00"
      /></FilterField
    >
    <FilterField
      >Tipo de actividad<select value={draft.kind} onchange={changeKind}>
        <option value="all">Audiencias y plazos</option><option value="hearing">Audiencias</option>
        <option value="deadline">Plazos</option>
        <option value="resource_hearing">Audiencias de recursos</option>
        <option value="precautionary_hearing">Audiencias cautelares</option>
      </select></FilterField
    >
    <FilterField
      >Estado de audiencia<select
        bind:value={draft.hearing_status}
        disabled={draft.kind === 'deadline'}
      >
        <option value="scheduled">Programadas</option>
        {#if draft.kind !== 'resource_hearing'}<option value="cancelled">Canceladas</option>{/if}
        <option value="all">Todas</option>
      </select></FilterField
    >
    <Button variant="primary" type="submit">Consultar Agenda</Button>
  </FilterBar>
  <MoreFilters label={'C\u00f3mo se cuenta el periodo'}>
    <p class="filter-note">
      El periodo empieza a las 00:00 en el desfase indicado y excluye el inicio del d&#237;a final.
      Las semanas empiezan el lunes. M&#225;ximo 366 d&#237;as; el estado s&#243;lo filtra
      audiencias ordinarias y cautelares. Las audiencias de recurso no tienen un estado de
      cancelaci&#243;n.
    </p>
  </MoreFilters>
</form>
