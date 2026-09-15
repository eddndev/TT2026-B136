<script>
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import AppendVersion from './AppendVersion.svelte';
  import VersionHistory from './VersionHistory.svelte';
  import VersionDetail from './VersionDetail.svelte';
  import { can } from '../lib/documents.mjs';
  export let api;
  export let user;
  export let document;
  export let onupdate;
  export let ondenied = () => {};
  let current = document;
  let selected = document;
  let versions = [];
  let firstAvailableVersion = 1;
  let nextBefore;
  let hasMore = false;
  let busy = false;
  let opening = false;
  let error = '';
  let unavailable = false;
  let append;
  let alive = true;
  let historyGeneration = 0;
  let detailGeneration = 0;
  function denyAccess(failure) {
    ondenied(failure);
    unavailable = true;
    selected = null;
    versions = [];
    hasMore = false;
    nextBefore = undefined;
    firstAvailableVersion = 1;
    historyGeneration++;
    detailGeneration++;
    busy = false;
    opening = false;
  }
  function updateCurrent(record) {
    if (!alive || record.version < current.version) return;
    current = record;
    onupdate(record);
  }
  async function history(more = false) {
    const request = ++historyGeneration;
    busy = true;
    error = '';
    if (!more) {
      versions = [];
      hasMore = false;
      nextBefore = undefined;
    }
    try {
      const page = await api.versions(document.id, {
        beforeVersion: more ? nextBefore : undefined,
      });
      if (!alive || request !== historyGeneration) return;
      versions = more ? [...versions, ...page.versions] : page.versions;
      hasMore = page.has_more;
      nextBefore = page.next_before_version;
      firstAvailableVersion = page.first_available_version;
      unavailable = false;
      if (versions[0]?.version > current.version) updateCurrent(versions[0]);
    } catch (failure) {
      if (alive && request === historyGeneration) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) denyAccess(failure);
      }
    } finally {
      if (alive && request === historyGeneration) busy = false;
    }
  }
  async function select(record) {
    const request = ++detailGeneration;
    opening = true;
    selected = null;
    error = '';
    const exact = api.version(document.id, record.version);
    try {
      const result = await exact.detail();
      if (alive && request === detailGeneration) selected = result;
    } catch (failure) {
      if (alive && request === detailGeneration) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) denyAccess(failure);
      }
    } finally {
      exact.dispose();
      if (alive && request === detailGeneration) opening = false;
    }
  }
  function update(record) {
    if (!alive || selected?.version !== record.version) return;
    const changed = selected.sealed !== record.sealed;
    selected = record;
    versions = versions.map((entry) =>
      entry.version === record.version ? { ...entry, sealed: record.sealed } : entry,
    );
    if (record.version === current.version) {
      current = record;
      onupdate(record);
    }
    if (changed) history();
  }
  function appended(record) {
    if (!alive) return;
    detailGeneration++;
    opening = false;
    current = record;
    selected = record;
    onupdate(record);
    history();
  }
  onMount(() => {
    history();
  });
  onDestroy(() => {
    alive = false;
    historyGeneration++;
    detailGeneration++;
  });
</script>

<div class="section-heading version-heading">
  <div>
    <span class="eyebrow">DOCUMENTO Y VERSIONES</span>
    <p class="hint">Cada versi&#243;n conserva su archivo, firma y sello.</p>
  </div>
  {#if !unavailable && can(user.role, 'documents')}<button
      class="primary"
      onclick={() => append.open()}><Icon name="plus" size={18} />Agregar versi&#243;n</button
    >{/if}
</div>
{#if !unavailable}<AppendVersion
    bind:this={append}
    {api}
    document={current}
    onappended={appended}
    {ondenied}
    oncurrent={(record) => {
      updateCurrent(record);
      history();
    }}
  />{/if}
<VersionHistory
  {versions}
  currentVersion={current.version}
  selectedVersion={selected?.version}
  {firstAvailableVersion}
  {hasMore}
  {busy}
  onselect={select}
  onmore={() => history(true)}
  onrefresh={() => history()}
/>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if opening}<p class="notice" role="status">Consultando la versi&#243;n seleccionada...</p>{/if}
{#if selected}<p class="selected-version-label">
    {selected.version === current.version
      ? `Consultando versi\u00f3n actual: ${selected.version}`
      : `Consultando versi\u00f3n hist\u00f3rica: ${selected.version}`}
  </p>
  {#key selected.version}<VersionDetail
      {api}
      {user}
      document={selected}
      onupdate={update}
      {ondenied}
    />{/key}{/if}
