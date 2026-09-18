<script>
  import { onDestroy } from 'svelte';
  import CalendarSources from './CalendarSources.svelte';
  import CalendarValues from './CalendarValues.svelte';
  import { loadDeadlineReference } from './deadline-reference-data.mjs';
  import { deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import {
    deadlineRuleLabel,
    deadlineRequirementLabel,
    deadlineBlockLabel,
  } from './deadline-view-labels.mjs';
  export let api, caseId, captured, ondenied;
  let value = null,
    busy = false,
    error = '',
    alive = true,
    generation = 0;
  let openCalendars = new Set();
  function toggleCalendar(id, open) {
    const next = new Set(openCalendars);
    if (open) next.add(id);
    else next.delete(id);
    openCalendars = next;
  }
  async function load() {
    const request = ++generation;
    busy = true;
    error = '';
    value = null;
    try {
      const result = await loadDeadlineReference(api, caseId, 'profile', captured);
      if (alive && request === generation) value = result;
    } catch (failure) {
      if (alive && request === generation) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<section aria-label="Perfil exacto del plazo">
  <button class="secondary" disabled={busy} onclick={load}
    >{busy ? 'Consultando perfil...' : 'Consultar perfil capturado'}</button
  >
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if value}{@const d = value.definition}
    <h3>{d.title} / Revisi&#243;n {value.revision}</h3>
    <p class="case-multiline">{d.description}</p>
    <p>
      {d.scope.kind === 'case' ? 'Perfil privado de este expediente' : 'Perfil de alcance global'}.
    </p>
    {#if d.scope.kind === 'global'}<p>
        {d.scope.value.title} / {d.scope.value.authority} / {d.scope.value.organ} / {d.scope.value
          .territory}
      </p>{/if}
    <p>Inicio requerido: {deadlineRequirementLabel(d.trigger)}</p>
    {#if d.template.kind === 'fixed'}<p>{deadlineRuleLabel(d.template.rule)}</p>
    {:else}<p>{deadlineRuleLabel(d.template.unit)}.</p>
      <p>
        M&#225;ximo del perfil: {d.template.maximum ?? 'Sin m\u00e1ximo declarado'}. La cantidad
        concedida se declara por separado.
      </p>{/if}
    <h4>Finalizaci&#243;n</h4>
    {#if d.completion.kind === 'civil_cutoff'}<p>
        Hora de corte: {d.completion.time}; desfase: {d.completion.offset_seconds} segundos.
      </p>
      <p>Canal: {d.completion.channel}. Cobertura: {d.completion.from} / {d.completion.through}.</p>
      <p>Referencia: {d.references.find((row) => row.id === d.completion.reference_id)?.title}</p>
    {:else}<p>
        {d.completion.kind === 'arithmetic_instant'
          ? 'Instante calculado por horas transcurridas.'
          : 'Fecha civil candidata, sin hora de corte.'}
      </p>{/if}
    <h4>Condiciones de aplicabilidad</h4>
    {#each d.conditions as condition}<div class="case-comparison">
        <p class="case-multiline">{condition.statement}</p>
        <p>
          Referencias: {condition.reference_ids
            .map((id) => d.references.find((row) => row.id === id)?.title || id)
            .join(' / ')}
        </p>
        <details>
          <summary>Identidad de la condici&#243;n</summary><code>{condition.id}</code>
        </details>
      </div>{/each}
    <CalendarSources sources={d.references} />
    <details>
      <summary>Ejemplos declarados del perfil ({d.examples.length})</summary>
      {#each d.examples as example}<article class="case-comparison">
          <p>Inicio: {factTimeLabel(example.anchor)}</p>
          <p>Cantidad concedida: {example.ordered_quantity ?? 'No declarada'}</p>
          {#if example.expected.kind === 'rule_blocked'}<p>
              {deadlineBlockLabel(example.expected.block)}
            </p>
          {:else if example.expected.outcome.kind === 'blocked'}<p>
              {deadlineBlockLabel(example.expected.outcome.block)}
            </p>
          {:else if example.expected.outcome.kind === 'civil_candidate'}<p>
              Fecha candidata: {example.expected.outcome.date}
            </p>
          {:else}<p>
              Instante candidato: {deadlineInstantLabel(example.expected.outcome.instant)}
            </p>{/if}
          <p>{example.locator}</p>
          <p>
            Referencias del ejemplo: {example.reference_ids
              .map((id) => d.references.find((row) => row.id === id)?.title || id)
              .join(' / ')}
          </p>
          {#if example.calendar}<details
              ontoggle={(event) => toggleCalendar(example.id, event.currentTarget.open)}
            >
              <summary>Calendario declarado del ejemplo</summary>
              {#if openCalendars.has(example.id)}<CalendarValues values={example.calendar} />{/if}
            </details>
          {:else}<p>Sin calendario declarado en este ejemplo.</p>{/if}
        </article>{/each}
    </details>
  {/if}
</section>
