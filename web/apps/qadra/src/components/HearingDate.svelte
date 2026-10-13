<script>
  export let label = 'Fecha y hora de la audiencia',
    value,
    disabled = false;
</script>

<fieldset class="case-offenses" {disabled}>
  <legend>{label}</legend>
  <div class="case-field-grid">
    <label
      >Fecha<input
        type="date"
        min="0001-01-01"
        max="9999-12-31"
        required
        bind:value={value.date}
      /></label
    >
    <label
      >Hora<input type="time" aria-label="Hora" step="1" required bind:value={value.time} />
      <small>Si indicas solo horas y minutos, los segundos se registran como 00.</small></label
    >
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
    Registra la fecha y hora comunicadas. Conservamos el desfase declarado sin convertirlo al de tu
    navegador.
  </p>
</fieldset>
