<script>
  export let label,
    value,
    disabled = false;
</script>

<fieldset class="case-offenses stage-date" {disabled}>
  <legend>{label}</legend>
  <div class="case-field-grid">
    <label
      >Precisi&#243;n<select aria-label="Precisi&#243;n" bind:value={value.precision}>
        <option value="date">Fecha sin hora conocida</option><option value="instant"
          >Fecha y hora conocidas</option
        >
      </select></label
    >
    <label>Fecha<input type="date" required bind:value={value.date} /></label>
    {#if value.precision === 'instant'}<label
        >Hora<input type="time" step="1" required bind:value={value.time} /></label
      >{/if}
    <label
      >Desfase UTC<input
        aria-label="Desfase UTC"
        required
        bind:value={value.offset}
        placeholder="-06:00"
        maxlength="6"
      />
      <small>Declara el desfase de esa fecha, de -14:00 a +14:00.</small></label
    >
  </div>
  <p class="hint">
    Una fecha sin hora conserva esa precisi&#243;n. No se asigna una hora autom&#225;ticamente.
  </p>
</fieldset>
