<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import MemberFilters from './MemberFilters.svelte';
  import MemberCard from './MemberCard.svelte';
  import MemberAccessEditor from './MemberAccessEditor.svelte';
  import { discardMemberAccess, freshMemberAccess } from '../lib/member-access-draft.mjs';
  export let api, user;
  const scoped = api.members();
  const session = getContext('session-drafts'),
    principalId = user.id;
  let rows = [],
    next = null,
    more = false,
    query = { limit: 20, status: 'active' };
  let busy = false,
    opening = false,
    selected = null,
    loaded = false,
    denied = false;
  let error = '',
    message = '',
    alive = true,
    generation = 0;
  const admitted = () =>
    alive &&
    (!session ||
      (session.principal()?.id === principalId &&
        session.principal()?.role === 'owner' &&
        session.canAdmit()));
  function deny(failure, id) {
    discardMemberAccess(session, principalId, failure, id);
    generation++;
    selected = null;
    busy = false;
    opening = false;
    error = failure.message;
    if (failure.status === 403) {
      denied = true;
      rows = [];
      more = false;
      next = null;
    } else rows = rows.filter((row) => row.id !== id);
  }
  export async function refresh() {
    if (alive) await load();
  }
  async function load(append = false) {
    if (!admitted() || denied || selected || (append && (busy || !more))) return;
    const request = ++generation;
    busy = true;
    error = '';
    if (!append) {
      rows = [];
      next = null;
      more = false;
      loaded = false;
    }
    try {
      const value = await scoped.list({ ...query, ...(append ? { cursor: next } : {}) });
      if (!admitted() || request !== generation) return;
      if (append && rows.length && value.items.length && rows.at(-1).id >= value.items[0].id)
        throw new Error(
          'La continuaci\u00f3n del directorio no conserva el orden. Vuelve a buscar.',
        );
      rows = append ? [...rows, ...value.items] : value.items;
      next = value.next_cursor;
      more = value.has_more;
      loaded = true;
    } catch (failure) {
      if (admitted() && request === generation) {
        error = failure.message;
        if (failure.status === 403) deny(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function open(row) {
    if (!admitted() || busy || opening || selected || denied) return;
    const request = ++generation;
    opening = true;
    error = '';
    message = '';
    try {
      const value = await freshMemberAccess({
        api,
        members: scoped,
        principalId,
        id: row.id,
        admitted: () => admitted() && request === generation,
      });
      if (value && admitted() && request === generation) selected = value;
    } catch (failure) {
      if (admitted() && request === generation) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) deny(failure, row.id);
      }
    } finally {
      if (alive && request === generation) opening = false;
    }
  }
  async function confirmed() {
    if (!alive || (session && session.principal()?.id !== principalId)) return;
    selected = null;
    message = 'Los permisos de la cuenta se confirmaron.';
    if (admitted()) await load();
  }
  onMount(() => {
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    rows = [];
    selected = null;
    scoped.dispose();
  });
</script>

<section
  class="card member-directory"
  aria-label="Directorio de cuentas"
  aria-busy={busy || opening}
>
  <div class="section-heading">
    <div>
      <h2>Directorio de cuentas</h2>
      <p class="hint">Consulta las cuentas del despacho y administra su acceso.</p>
    </div>
  </div>
  <MemberFilters
    {busy}
    disabled={denied || opening || !!selected}
    onapply={(value) => {
      query = value;
      message = '';
      load();
    }}
  />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if message}<p class="notice success" role="status">{message}</p>{/if}
  {#if busy}<p role="status">Consultando cuentas...</p>{/if}
  {#if opening}<p role="status">Consultando el acceso actual...</p>{/if}
  {#each rows as record (record.id)}<MemberCard
      {record}
      onopen={open}
      disabled={busy || opening || !!selected || denied}
    />{/each}
  {#if loaded && !busy && !error && !rows.length}<p class="hint">
      No hay cuentas para estos filtros.
    </p>{/if}
  {#if more}<div class="action-row">
      <button
        class="secondary"
        disabled={busy || opening || !!selected || denied}
        onclick={() => load(true)}>Cargar m&#225;s cuentas</button
      >
    </div>{/if}
</section>
{#if selected}{#key selected.id}<MemberAccessEditor
      {api}
      {user}
      record={selected}
      onconfirmed={confirmed}
      oncancel={() => {
        selected = null;
      }}
      ondenied={deny}
    />{/key}{/if}

<style>
  .member-directory {
    margin-bottom: 24px;
  }
</style>
