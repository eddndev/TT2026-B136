<script>
  import { onDestroy } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  export let api, document, onupdate;
  export let disabled = false,
    busy = '',
    error = '',
    message = '',
    uncertain = false;
  export let ondenied = () => {};
  let alive = true,
    confirmation = false;
  const reference = () => ({
    case_id: document.case_id,
    id: document.id,
    version: document.version,
  });
  const matches = (value, expected) =>
    value.case_id === expected.case_id &&
    value.id === expected.id &&
    value.version === expected.version;
  export function open() {
    if (!busy && !disabled && !uncertain && !$administration.closed) confirmation = true;
  }
  function fail(failure) {
    error = failure.message;
    if ([403, 404].includes(failure.status)) ondenied(failure);
  }
  async function seal() {
    if (busy || disabled || uncertain || !confirmation || $administration.closed) return;
    const selected = reference();
    busy = 'seal';
    error = '';
    message = '';
    uncertain = true;
    let confirmed = false;
    try {
      const result = await api.seal(document.id);
      if (!alive || !matches(document, selected)) return;
      confirmed = true;
      uncertain = false;
      confirmation = false;
      await onupdate({ ...document, ...result, report: null });
      if (alive) message = 'Documento sellado correctamente.';
    } catch (failure) {
      if (!alive || !matches(document, selected)) return;
      uncertain = !confirmed && (!failure.status || failure.status >= 500);
      fail(failure);
    } finally {
      if (alive) busy = '';
    }
  }
  async function refresh() {
    if (busy || disabled || !uncertain) return;
    const selected = reference();
    busy = 'seal-status';
    error = '';
    message = '';
    try {
      const result = await api.detail(document.id);
      if (!alive || !matches(document, selected)) return;
      if (!matches(result, selected) || typeof result.sealed !== 'boolean')
        throw new Error('La respuesta no corresponde al estado de la versi\u00f3n seleccionada.');
      await onupdate({ ...document, ...result, report: null });
      if (!alive) return;
      uncertain = false;
      confirmation = false;
      message = result.sealed
        ? 'El estado actual indica que esta versi\u00f3n est\u00e1 sellada.'
        : 'Esta versi\u00f3n sigue sin sello. Puedes iniciar una nueva confirmaci\u00f3n.';
    } catch (failure) {
      if (alive && matches(document, selected)) fail(failure);
    } finally {
      if (alive) busy = '';
    }
  }
  onDestroy(() => {
    alive = false;
    if (busy === 'seal' || busy === 'seal-status') busy = '';
  });
</script>

{#if confirmation}<div class="notice stack seal-confirmation">
    <strong>&#191;Firmar y sellar este documento?</strong>
    <p>
      Se registrar&#225; la operaci&#243;n con tu identidad y se generar&#225; evidencia con la
      autoridad de sellado local. Esta acci&#243;n no se puede deshacer desde la interfaz.
    </p>
    <div class="action-row">
      <button
        class="primary"
        disabled={disabled || !!busy || uncertain || $administration.closed}
        onclick={seal}>{busy === 'seal' ? 'Sellando...' : 'Confirmar sellado'}</button
      ><button class="secondary" disabled={!!busy} onclick={() => (confirmation = false)}
        >Cancelar</button
      >
    </div>
  </div>{/if}
{#if uncertain}<div class="notice stack">
    <p>
      El resultado del sellado es incierto. Consulta esta versi&#243;n antes de otra decisi&#243;n.
    </p>
    <button class="secondary" disabled={disabled || !!busy} onclick={refresh}
      >Consultar estado del sellado</button
    >
  </div>{/if}
