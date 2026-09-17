<script>
  export let value,
    label,
    disabled = false;
  const pad = (n) => String(n).padStart(2, '0');
  const dateText = (v) =>
    Number.isInteger(v?.year) && Number.isInteger(v.month) && Number.isInteger(v.day)
      ? `${String(v.year).padStart(4, '0')}-${pad(v.month)}-${pad(v.day)}`
      : '';
  const clockText = (v) =>
    Number.isInteger(v?.hour) &&
    Number.isInteger(v.minute) &&
    (v.precision !== 'second' || Number.isInteger(v.second))
      ? `${pad(v.hour)}:${pad(v.minute)}${v.precision === 'second' ? `:${pad(v.second)}` : ''}`
      : '';
  const offsetText = (n) =>
    Number.isInteger(n)
      ? `${n < 0 ? '-' : '+'}${pad(Math.floor(Math.abs(n) / 3600))}:${pad((Math.abs(n) % 3600) / 60)}`
      : '';
  function precision(next) {
    const old = value || {};
    if (!next || next === 'unknown') {
      value = { precision: next };
      return;
    }
    value = {
      precision: next,
      year: old.year,
      month: old.month,
      day: old.day,
      offset_seconds: old.offset_seconds ?? null,
    };
    if (['minute', 'second'].includes(next))
      value = { ...value, hour: old.hour, minute: old.minute };
    if (next === 'second') value = { ...value, second: old.second };
  }
  function date(raw) {
    const parts = /^(\d{4})-(\d{2})-(\d{2})$/.exec(raw);
    value = {
      ...value,
      year: parts ? Number(parts[1]) : undefined,
      month: parts ? Number(parts[2]) : undefined,
      day: parts ? Number(parts[3]) : undefined,
    };
  }
  function clock(raw) {
    const parts = raw.split(':');
    value = {
      ...value,
      hour: parts[0] ? Number(parts[0]) : undefined,
      minute: parts[1] ? Number(parts[1]) : undefined,
    };
    if (value.precision === 'second')
      value = { ...value, second: parts[2] ? Number(parts[2]) : undefined };
  }
  function offset(raw) {
    const parts = /^([+-])(\d{2}):(\d{2})$/.exec(raw);
    const total =
      parts && Number(parts[3]) < 60
        ? (Number(parts[2]) * 60 + Number(parts[3])) * 60 * (parts[1] === '-' ? -1 : 1)
        : NaN;
    value = { ...value, offset_seconds: total };
  }
</script>

<fieldset class="case-offenses fact-time-fields" {disabled}>
  <legend>Tiempo de {label}</legend>
  <label
    >Precisi&#243;n de {label}<select
      value={value?.precision || ''}
      onchange={(event) => precision(event.currentTarget.value)}
    >
      <option value="">Selecciona la precisi&#243;n</option><option value="unknown"
        >No consta</option
      >
      <option value="date">Solo fecha</option><option value="minute">Fecha y minuto</option><option
        value="second">Fecha y segundo</option
      >
    </select></label
  >
  {#if value?.precision && value.precision !== 'unknown'}
    <div class="case-field-grid">
      <label
        >Fecha de {label}<input
          type="date"
          min="0001-01-01"
          max="9999-12-31"
          value={dateText(value)}
          oninput={(event) => date(event.currentTarget.value)}
        /></label
      >
      {#if ['minute', 'second'].includes(value.precision)}<label
          >Hora de {label}<input
            type="time"
            step={value.precision === 'second' ? 1 : 60}
            value={clockText(value)}
            oninput={(event) => clock(event.currentTarget.value)}
          /></label
        >{/if}
      <label
        >Desfase de {label}<select
          value={value.offset_seconds === null ? 'absent' : 'declared'}
          onchange={(event) =>
            (value = {
              ...value,
              offset_seconds: event.currentTarget.value === 'absent' ? null : NaN,
            })}
        >
          <option value="absent">No declarado</option><option value="declared"
            >Declarado expresamente</option
          >
        </select></label
      >
      {#if value.offset_seconds !== null}<label
          >Desfase UTC de {label}<input
            placeholder="-06:00"
            value={offsetText(value.offset_seconds)}
            onchange={(event) => offset(event.currentTarget.value)}
            aria-invalid={!Number.isInteger(value.offset_seconds) ||
              Math.abs(value.offset_seconds) > 50400}
          /></label
        >{/if}
    </div>
  {/if}
  <p class="hint">
    Conserva solo los componentes declarados. No se completa hora, segundo ni desfase.
  </p>
</fieldset>
