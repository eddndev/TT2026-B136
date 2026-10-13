<script>
  import CalendarSources from './CalendarSources.svelte';
  import {
    calendarEntities,
    calendarStates,
    calendarWeekdays,
  } from '../lib/judicial-calendar-labels.mjs';
  export let values;
  const entityName = (code) => calendarEntities.find((row) => row[0] === code)?.[1] || code;
  function sourceNames(ids) {
    return ids.map((id) => values.sources.find((s) => s.id === id)?.title || id).join(' / ');
  }
</script>

<div class="calendar-values">
  <dl class="calendar-description">
    <div>
      <dt>T&#237;tulo del calendario</dt>
      <dd>{values.scope.title}</dd>
    </div>
    <div>
      <dt>Fuero</dt>
      <dd>{values.scope.jurisdiction === 'federal' ? 'Federal' : 'Local'}</dd>
    </div>
    <div>
      <dt>Entidades</dt>
      <dd>{values.scope.entity_codes.map((code) => `${code} ${entityName(code)}`).join(', ')}</dd>
    </div>
    <div>
      <dt>Autoridad</dt>
      <dd>{values.scope.authority}</dd>
    </div>
    <div>
      <dt>&#211;rgano</dt>
      <dd>{values.scope.organ}</dd>
    </div>
    <div>
      <dt>Territorio</dt>
      <dd>{values.scope.territory}</dd>
    </div>
    <div>
      <dt>Uso declarado</dt>
      <dd class="case-multiline">{values.scope.use_description}</dd>
    </div>
    <div>
      <dt>Cobertura inclusiva</dt>
      <dd>{values.coverage.from} / {values.coverage.through}</dd>
    </div>
  </dl>
  <details>
    <summary>Patr&#243;n semanal y excepciones de esta revisi&#243;n</summary>
    <div class="calendar-rule-summary">
      {#each values.weekly_pattern as rule}<article>
          <h4>{calendarWeekdays[rule.weekday - 1]} / {calendarStates[rule.classification]}</h4>
          <p class="case-multiline">{rule.explanation}</p>
          <p>Fuentes: {sourceNames(rule.source_ids) || 'Ninguna declarada'}</p>
        </article>{/each}
    </div>
    {#each values.exceptions as rule}<article class="calendar-source-row">
        <h4>{rule.from} / {rule.through}: {calendarStates[rule.classification]}</h4>
        <p class="case-multiline">{rule.explanation}</p>
        <p>Fuentes: {sourceNames(rule.source_ids) || 'Ninguna declarada'}</p>
        <details><summary>Identidad de la excepci&#243;n</summary><code>{rule.id}</code></details>
      </article>{/each}
    {#if !values.exceptions.length}<p>Sin excepciones declaradas.</p>{/if}
  </details>
  <CalendarSources sources={values.sources} />
</div>
