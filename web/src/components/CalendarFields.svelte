<script>
  import CalendarScopeFields from './CalendarScopeFields.svelte';
  import CalendarSourceFields from './CalendarSourceFields.svelte';
  import CalendarRuleFields from './CalendarRuleFields.svelte';
  import CalendarExceptionFields from './CalendarExceptionFields.svelte';
  import { calendarWeekdays } from '../lib/judicial-calendar-labels.mjs';
  export let draft,
    disabled = false,
    immutableScope = false;
  $: usedIds = [...draft.weekly_pattern, ...draft.exceptions].flatMap((rule) => rule.source_ids);
</script>

<CalendarScopeFields bind:scope={draft.scope} disabled={disabled || immutableScope} />
<fieldset class="calendar-fieldset" {disabled}>
  <legend>Cobertura civil</legend>
  <div class="calendar-form-grid">
    <label
      >Cobertura desde<input
        type="date"
        min="0001-01-01"
        max="9999-12-31"
        bind:value={draft.coverage.from}
      /></label
    >
    <label
      >Cobertura hasta<input
        type="date"
        min="0001-01-01"
        max="9999-12-31"
        bind:value={draft.coverage.through}
      /></label
    >
  </div>
  <p class="hint">
    De 1 a 1096 d&#237;as inclusivos; no se infieren reglas fuera de esta cobertura.
  </p>
</fieldset>
<CalendarSourceFields bind:sources={draft.sources} {usedIds} {disabled} />
<section class="calendar-fields-section" aria-label="Patr&#243;n semanal">
  <h3>Patr&#243;n semanal expreso</h3>
  <p class="hint">
    Declara cada d&#237;a. Sin resolver conserva la incertidumbre; no equivale a excluido.
  </p>
  <div class="calendar-form-grid">
    {#each draft.weekly_pattern as rule (rule.weekday)}<fieldset
        class="calendar-fieldset"
        {disabled}
      >
        <legend>Regla de {calendarWeekdays[rule.weekday - 1]}</legend><CalendarRuleFields
          bind:rule
          sources={draft.sources}
          {disabled}
        />
      </fieldset>{/each}
  </div>
</section>
<CalendarExceptionFields bind:exceptions={draft.exceptions} sources={draft.sources} {disabled} />
