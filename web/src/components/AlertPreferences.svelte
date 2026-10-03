<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import AlertPreferenceFields from './AlertPreferenceFields.svelte';
  import AlertPreferenceSummary from './AlertPreferenceSummary.svelte';
  import { alertPreferenceCommand } from '../lib/alerts-preference-values.mjs';
  import { same } from '../lib/alerts-primitives.mjs';
  import { alertFailure } from '../lib/alerts-presentation.mjs';
  import {
    createAlertPreferenceDraft,
    freshAlertPreferences,
  } from '../lib/alert-preference-draft.mjs';
  export let api, principalId, authorize, onconfirmed, oncancel, ondenied;
  const session = getContext('session-drafts');
  let current = null,
    values = null,
    hours = {},
    candidate = null,
    command = null;
  let mode = 'editing',
    busy = false,
    alive = true,
    completed = false,
    generation = 0,
    error = '';
  const recovery = session ? createAlertPreferenceDraft({ session, principalId, capture }) : null;
  const admitted = () => alive && !completed && (!recovery || recovery.admitted());
  const active = (request) => request === generation && admitted();
  $: frozen = mode === 'uncertain' || mode === 'not_observed';
  function capture() {
    return structuredClone({
      base: current,
      values,
      hours,
      command,
      mode: mode === 'not_observed' ? 'uncertain' : mode,
    });
  }
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
    if ([403, 404].includes(failure.status)) {
      recovery?.deny();
      completed = true;
      generation++;
      ondenied(failure);
    }
  }
  function close() {
    if (busy || !admitted()) return;
    recovery?.close();
    completed = true;
    generation++;
    oncancel();
  }
  function confirmed(value) {
    recovery?.close();
    completed = true;
    generation++;
    onconfirmed(value);
  }
  function fresh(request) {
    return freshAlertPreferences({
      api,
      authorize,
      principalId,
      admitted: () => active(request),
    });
  }
  function restore(value, latest) {
    current = value.base;
    values = value.values;
    hours = value.hours;
    command = value.command;
    candidate = null;
    mode =
      value.mode === 'editing' && latest.revision !== current.revision ? 'conflict' : value.mode;
  }
  async function load() {
    if (busy || !admitted()) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const saved = recovery?.pending();
      if (saved) {
        const outcome = await recovery.restore(saved, () => fresh(request), restore);
        if (!active(request)) return;
        if (outcome.status !== 'restored')
          throw new Error(
            'No fue posible recuperar el borrador. Vuelve a consultar las preferencias.',
          );
      } else {
        const result = await fresh(request);
        if (!result || !active(request)) return;
        apply(result);
        recovery?.register(current.revision);
      }
    } catch (failure) {
      if (active(request)) {
        error = alertFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
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
    if (
      busy ||
      !admitted() ||
      !current ||
      mode === 'uncertain' ||
      (mode === 'conflict' && !candidate)
    )
      return;
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
    const request = ++generation;
    busy = true;
    mode = 'uncertain';
    try {
      const result = await api.savePreferences(command);
      if (active(request)) confirmed(result.preferences);
    } catch (failure) {
      if (!active(request)) return;
      error = alertFailure(failure);
      if (failure.status === 409) {
        mode = 'conflict';
        candidate = null;
      } else if ([400, 422].includes(failure.status)) {
        mode = 'editing';
        command = null;
      } else mode = 'uncertain';
      denied(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function check() {
    if (busy || !admitted() || !current) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const result = await fresh(request);
      if (!result || !active(request)) return;
      if (
        mode === 'uncertain' &&
        result.receipt?.operation_id === command.operation_id &&
        result.receipt.expected_revision === command.expected_revision &&
        result.revision === command.expected_revision + 1 &&
        result.user_id === principalId &&
        same(result.values, command.values)
      ) {
        confirmed(result);
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
      if (active(request)) {
        error = alertFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  onMount(load);
  onDestroy(() => {
    alive = false;
    generation++;
    recovery?.dispose();
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
        <button class="secondary" type="button" disabled={busy} onclick={close}>Cancelar</button>
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
      <button class="text-button" onclick={close}>Cerrar preferencias</button>
    </div>{/if}
</section>
