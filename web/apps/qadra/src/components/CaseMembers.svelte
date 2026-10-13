<script>
  import { onMount, onDestroy } from 'svelte';
  import MemberFilters from './MemberFilters.svelte';
  import MemberCard from './MemberCard.svelte';
  export let api, caseId, ondenied;
  const scoped = api.caseMembers(caseId);
  let query = { limit: 20, selection: 'assigned' },
    rows = [],
    more = false,
    next = null;
  let busy = false,
    writing = false,
    loaded = false,
    denied = false,
    choice = null,
    uncertain = false;
  let error = '',
    message = '',
    alive = true,
    generation = 0;
  function fail(failure) {
    error = failure.message;
    if (failure.status === 403 || failure.code === 'case_not_found') {
      generation++;
      rows = [];
      choice = null;
      more = false;
      next = null;
      busy = false;
      writing = false;
      denied = true;
      ondenied(failure);
    }
  }
  async function load(append = false) {
    if (denied || (append && (busy || !more))) return;
    const request = ++generation;
    busy = true;
    error = '';
    if (!append) {
      rows = [];
      more = false;
      next = null;
      loaded = false;
    }
    try {
      const page = await scoped.list({ ...query, ...(append ? { cursor: next } : {}) });
      if (!alive || request !== generation) return;
      if (append && rows.length && page.items.length && rows.at(-1).id >= page.items[0].id)
        throw new Error(
          'La continuaci\u00f3n de asignaciones no conserva el orden. Vuelve a buscar.',
        );
      rows = append ? [...rows, ...page.items] : page.items;
      more = page.has_more;
      next = page.next_cursor;
      loaded = true;
    } catch (failure) {
      if (alive && request === generation) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function select(row) {
    if (busy || writing || denied || choice) return;
    error = '';
    message = '';
    choice = { record: structuredClone(row), selection: query.selection };
    uncertain = false;
  }
  async function confirm() {
    if (!choice || busy || writing || uncertain || denied) return;
    writing = true;
    error = '';
    try {
      if (choice.selection === 'available') await scoped.assign(choice.record.id);
      else await scoped.remove(choice.record.id);
      if (!alive) return;
      message =
        choice.selection === 'available'
          ? 'Asignaci\u00f3n confirmada.'
          : 'Retiro de asignaci\u00f3n confirmado.';
      choice = null;
      await load();
    } catch (failure) {
      if (alive) {
        fail(failure);
        if (!failure.status || failure.status >= 500) uncertain = true;
        else if (!denied) choice = null;
      }
    } finally {
      if (alive) writing = false;
    }
  }
  function reconcile() {
    if (writing || busy) return;
    choice = null;
    uncertain = false;
    message = 'Consulta las asignaciones actuales antes de decidir otra acci\u00f3n.';
    load();
  }
  onMount(() => {
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    rows = [];
    choice = null;
    scoped.dispose();
  });
</script>

<section aria-label="Asignaciones" aria-busy={busy || writing}>
  <div class="page-heading">
    <div>
      <span class="eyebrow">ACCESO AL EXPEDIENTE</span>
      <h1>Asignaciones</h1>
      <p>Selecciona cuentas del despacho por correo y rol.</p>
    </div>
  </div>
  <p class="notice">
    Los administradores tienen acceso global aunque no figuren asignados. El cierre administrativo
    conserva la administraci&#243;n de asignaciones.
  </p>
  <div class="card">
    <MemberFilters
      caseScope
      {busy}
      disabled={denied || writing || !!choice}
      onapply={(value) => {
        query = value;
        message = '';
        load();
      }}
    />
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if message}<p class="notice success" role="status">{message}</p>{/if}
    {#if busy}<p role="status">Consultando asignaciones...</p>{/if}
    {#if query.selection === 'assigned'}<p class="hint">
        Las cuentas inactivas conservan su asignaci&#243;n, pero no pueden acceder. Reactivar exige
        un nuevo inicio de sesi&#243;n.
      </p>{/if}
    {#each rows as record (record.id)}<MemberCard
        {record}
        assignment={query.selection}
        disabled={busy || writing || denied || !!choice}
        onopen={select}
      />{/each}
    {#if loaded && !busy && !error && !rows.length}<p class="hint">
        No hay cuentas para esta selecci&#243;n y filtros.
      </p>{/if}
    {#if more}<div class="action-row">
        <button
          class="secondary"
          disabled={busy || writing || denied || !!choice}
          onclick={() => load(true)}>Cargar m&#225;s asignaciones</button
        >
      </div>{/if}
    {#if choice}<div class="notice member-confirmation">
        <h2>{choice.selection === 'available' ? 'Asignar cuenta' : 'Retirar asignaci\u00f3n'}</h2>
        <p>{choice.record.email}</p>
        <p>
          {choice.selection === 'available'
            ? 'La cuenta acceder\u00e1 conforme a los permisos de su rol.'
            : 'Se retirar\u00e1 esta asignaci\u00f3n; la cuenta y las capturas hist\u00f3ricas se conservan.'}
        </p>
        {#if uncertain}<p>
            No se confirm&#243; el resultado. Consulta la lista antes de realizar otra acci&#243;n.
          </p>
          <button class="secondary" disabled={busy || writing} onclick={reconcile}
            >Consultar asignaciones actuales</button
          >
        {:else}<div class="action-row">
            <button class="primary" disabled={busy || writing} onclick={confirm}>
              {choice.selection === 'available'
                ? 'Confirmar asignaci\u00f3n'
                : 'Confirmar retiro'}</button
            >
            <button
              class="secondary"
              disabled={busy || writing}
              onclick={() => {
                choice = null;
              }}>Cancelar</button
            >
          </div>{/if}
      </div>{/if}
  </div>
</section>

<style>
  .member-confirmation {
    margin-top: 22px;
  }
  .member-confirmation p {
    overflow-wrap: anywhere;
  }
</style>
