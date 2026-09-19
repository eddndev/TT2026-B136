<script>
  import { onMount, onDestroy } from 'svelte';
  import AlertPreferenceFields from './AlertPreferenceFields.svelte';
  import AlertPreferenceSummary from './AlertPreferenceSummary.svelte';
  import { alertPreferenceCommand } from '../lib/alerts-preference-values.mjs';
  import { same } from '../lib/alerts-primitives.mjs';
  import { alertFailure } from '../lib/alerts-presentation.mjs';
  export let api, onconfirmed, oncancel, ondenied;
  let current = null,
    values = null,
    hours = {},
    candidate = null,
    command = null;
  let mode = 'editing',
    busy = false,
    alive = true,
    error = '';
  $: frozen = mode === 'uncertain' || mode === 'not_observed';
  function apply(value) {
    current = value;
    values = structuredClone(value.values);
    hours = Object.fromEntries(
      ['hearing_upcoming', 'deadline_upcoming'].map((key) => [
        key,
        values[key].lead_hours.join(','),
      ]),
    );
  }
  function denied(failure) {
    if ([403, 404].includes(failure.status)) ondenied(failure);
  }
  async function load() {
    busy = true;
    try {
      const result = await api.preferences();
      if (alive) apply(result.preferences);
    } catch (failure) {
      if (alive) {
        error = alertFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function leadHours(text) {
    if (!text.trim()) return [];
    const parts = text.split(',').map((value) => value.trim());
    if (parts.some((value) => !/^\d+$/.test(value)))
      throw new Error('Escribe horas enteras separadas por comas.');
    return parts.map(Number);
  }
  async function save(event) {
    event?.preventDefault();
    if (busy || !current || mode === 'uncertain' || (mode === 'conflict' && !candidate)) return;
    error = '';
    if (mode !== 'not_observed') {
      try {
        const submitted = structuredClone(values);
        for (const key of ['hearing_upcoming', 'deadline_upcoming'])
          submitted[key].lead_hours = leadHours(hours[key]);
        command = alertPreferenceCommand({
          operation_id: crypto.randomUUID(),
          expected_revision: candidate?.revision ?? current.revision,
          values: submitted,
        });
      } catch (failure) {
        error = alertFailure(failure);
        return;
      }
    }
    busy = true;
    try {
      const result = await api.savePreferences(command);
      if (alive) onconfirmed(result.preferences);
    } catch (failure) {
      if (!alive) return;
      error = alertFailure(failure);
      if (failure.status === 409) {
        mode = 'conflict';
        candidate = null;
      } else if ([400, 422].includes(failure.status)) mode = 'editing';
      else mode = 'uncertain';
      denied(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const result = (await api.preferences()).preferences;
      if (!alive) return;
      if (
        mode === 'uncertain' &&
        result.receipt?.operation_id === command.operation_id &&
        result.receipt.expected_revision === command.expected_revision &&
        same(result.values, command.values)
      ) {
        onconfirmed(result);
        return;
      }
      if (mode === 'uncertain' && result.revision === command.expected_revision) {
        mode = 'not_observed';
        error =
          'La consulta no confirma este guardado. Puedes reintentar explicitamente la misma operacion.';
      } else {
        candidate = result;
        mode = 'conflict';
      }
    } catch (failure) {
      if (alive) {
        error = alertFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  onMount(load);
  onDestroy(() => {
    alive = false;
  });
</script>

<section class="card alert-preferences" aria-label="Preferencias de alertas" aria-busy={busy}>
  <div class="section-heading">
    <h2>Preferencias de alertas</h2>
    <span class="badge info">Para mi cuenta</span>
  </div>
  <p>
    Define las anticipaciones y los canales de tus avisos. Leer una alerta no declara atenci&#243;n
    del plazo.
  </p>
  {#if current?.email_transport === 'disabled'}<p class="notice">
      El correo est&#225; deshabilitado en este servidor. Tus preferencias quedan guardadas.
    </p>{/if}
  <p class="hint">
    El correo contiene un aviso gen&#233;rico y el acceso a Qadra, sin datos del expediente.
  </p>
  {#if !current && busy}<p role="status">Consultando preferencias...</p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if current}
    <form class="stack" onsubmit={save}>
      <AlertPreferenceFields bind:values bind:hours disabled={busy || frozen} />
      {#if mode === 'conflict'}<button
          class="secondary"
          type="button"
          disabled={busy}
          onclick={check}>Consultar preferencias actuales</button
        >{/if}
      {#if mode === 'uncertain'}<button
          class="secondary"
          type="button"
          disabled={busy}
          onclick={check}>Comprobar guardado</button
        >{/if}
      {#if candidate}<AlertPreferenceSummary current={candidate} />{/if}
      <div class="action-row">
        <button class="secondary" type="button" disabled={busy} onclick={oncancel}>Cancelar</button>
        <button
          class="primary"
          disabled={busy || mode === 'uncertain' || (mode === 'conflict' && !candidate)}
        >
          {busy
            ? 'Guardando...'
            : mode === 'not_observed'
              ? 'Reintentar este guardado'
              : candidate
                ? 'Guardar mis preferencias'
                : 'Guardar preferencias'}
        </button>
      </div>
    </form>
  {:else if !busy}<div class="action-row">
      <button class="secondary" onclick={load}>Consultar preferencias</button>
      <button class="text-button" onclick={oncancel}>Cerrar preferencias</button>
    </div>{/if}
</section>
