<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import ResourceHearingEditor from './ResourceHearingEditor.svelte';
  import ResourceHearingCollection from './ResourceHearingCollection.svelte';
  import { canResources, resourceDenied } from '../lib/procedural-resource-errors.mjs';
  import { pendingResourceHearingDrafts } from '../lib/resource-hearing-draft.mjs';
  import {
    compatibleHearingKind,
    resourceHearingFailure,
  } from './resource-hearing-editor-values.mjs';
  export let api,
    user,
    caseId,
    resource,
    ondenied,
    oncreated = () => {},
    disabled = false,
    pending = false;
  const session = getContext('session-drafts'),
    resources = api.caseResources(caseId),
    administration = api.caseAdministration(caseId),
    actor = { id: user.id, email: user.email, role: user.role };
  const pendingDrafts = () => pendingResourceHearingDrafts(session, caseId, resource.id);
  let drafts = pendingDrafts(),
    savedDraft = null,
    head = null,
    closed = false,
    alive = true,
    initialized = false,
    opening = false,
    opened = false,
    editorBusy = false,
    listBusy = false,
    error = '',
    created = null,
    generation = 0,
    editorKey = 0;
  function admitted() {
    const current = session?.principal() || user;
    return (
      alive &&
      (!session || session.canAdmit()) &&
      current?.id === actor.id &&
      current?.email === actor.email &&
      current?.role === actor.role
    );
  }
  async function context(request) {
    const record = session ? await session.authorizeCase(caseId) : await administration.get();
    if (!admitted() || request !== generation) return null;
    if (
      record.id !== caseId ||
      !['active', 'closed'].includes(record.administration?.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente de la audiencia.');
    const current = await resources.get(resource.id);
    if (!admitted() || request !== generation) return null;
    return { head: current, closed: record.administration.administrative_status === 'closed' };
  }
  function fail(failure) {
    error = resourceHearingFailure(failure);
    if (resourceDenied(failure)) {
      generation++;
      head = created = null;
      opened = false;
      if (failure.status === 403 || failure.code === 'case_not_found')
        session?.registry.denyContext(caseId);
      ondenied(failure);
    }
  }
  async function initialize() {
    const request = ++generation;
    opening = true;
    try {
      const value = await context(request);
      if (!value) return;
      ({ head, closed } = value);
      initialized = true;
    } catch (failure) {
      if (admitted() && request === generation) fail(failure);
    } finally {
      if (alive) opening = false;
    }
  }
  async function open(saved = null) {
    if (pending || disabled || !admitted() || !canResources(user.role, 'manage')) return;
    const request = ++generation;
    opening = true;
    error = '';
    try {
      const value = await context(request);
      if (!value) return;
      ({ head, closed } = value);
      if (!saved && (closed || head.status !== 'active' || !compatible)) {
        error = 'La programacion requiere un caso activo y un recurso escrito compatible y activo.';
        return;
      }
      savedDraft = saved;
      editorKey++;
      opened = true;
    } catch (failure) {
      if (admitted() && request === generation) fail(failure);
    } finally {
      if (alive) opening = false;
    }
  }
  function close() {
    opened = false;
    savedDraft = null;
    editorBusy = false;
    drafts = pendingDrafts();
  }
  async function confirmed(value) {
    if (!admitted()) return;
    close();
    created = value;
    opening = true;
    try {
      await oncreated(value);
    } catch (failure) {
      if (admitted())
        error = 'La audiencia y su vinculo se registraron. Consulta el resultado sin reenviar.';
    } finally {
      if (alive) opening = false;
    }
  }
  $: compatible =
    !!head &&
    !!compatibleHearingKind(resource) &&
    compatibleHearingKind(resource) === compatibleHearingKind(head);
  $: pending = opening || editorBusy || listBusy || opened;
  onMount(initialize);
  onDestroy(() => {
    alive = false;
    generation++;
    pending = false;
    resources.dispose();
    administration.dispose();
  });
</script>

<div class="stack resource-hearing-scheduling">
  {#if canResources(user.role, 'manage')}
    <div class="action-row">
      <button
        class="primary"
        disabled={disabled ||
          pending ||
          !initialized ||
          closed ||
          head?.status !== 'active' ||
          !compatible}
        onclick={() => open()}>Crear audiencia de recurso</button
      >
      {#if !opened}{#each drafts as saved (saved.key)}
          <button
            class="secondary"
            disabled={disabled || pending || !initialized}
            onclick={() => open(saved)}>Retomar borrador de audiencia</button
          >
        {/each}{/if}
    </div>
    {#if initialized && !compatible}<p class="hint">
        La programacion propia admite apelacion escrita o revocacion escrita, con modalidad
        declarada en el recurso y en su cabeza actual.
      </p>{/if}
  {/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if opened}{#key editorKey}
      <ResourceHearingEditor
        {api}
        {user}
        {caseId}
        {resource}
        {head}
        {savedDraft}
        {disabled}
        ondenied={fail}
        onconfirmed={confirmed}
        oncancel={close}
        bind:pending={editorBusy}
      />
    {/key}{/if}
  <ResourceHearingCollection
    {api}
    {user}
    {caseId}
    {resource}
    ondenied={fail}
    {created}
    disabled={disabled || opened || opening}
    bind:pending={listBusy}
  />
</div>
