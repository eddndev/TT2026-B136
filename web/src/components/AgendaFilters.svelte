<script>
  export let value,
    onapply,
    disabled = false;
  let draft = { ...value };
</script>

<form
  class="hearing-agenda-filters"
  onsubmit={(event) => {
    event.preventDefault();
    onapply({ ...draft });
  }}
>
  <fieldset class="case-offenses" {disabled}>
    <legend>Rango de consulta</legend>
    <div class="case-field-grid">
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
      <label
        >Desfase de consulta<input
          required
          maxlength="6"
          bind:value={draft.offset}
          placeholder="+00:00"
        /></label
      >
      <label
        >Estado de audiencia<select bind:value={draft.status}
          ><option value="scheduled">Programadas</option><option value="cancelled"
            >Canceladas</option
          ><option value="all">Todas</option></select
        ></label
      >
    </div>
    <p class="hint">
      Desde las 00:00 del primer d&#237;a hasta las 00:00 del &#250;ltimo, sin incluirlo.
      M&#225;ximo 366 d&#237;as.
    </p>
    <button class="primary" type="submit">Consultar Agenda</button>
  </fieldset>
</form>
