<script>
  import Icon from './Icon.svelte';
  export let versions = [];
  export let currentVersion;
  export let selectedVersion;
  export let firstAvailableVersion = 1;
  export let hasMore = false;
  export let busy = false;
  export let onselect;
  export let onmore;
  export let onrefresh;
</script>

<section class="card version-history" aria-label="Historial de versiones" aria-busy={busy}>
  <div class="section-heading">
    <div>
      <h2>Historial de versiones</h2>
      <p class="hint">Versi&#243;n actual: {currentVersion}</p>
    </div>
    <button class="secondary" disabled={busy} onclick={onrefresh}>Actualizar historial</button>
  </div>
  {#if firstAvailableVersion > 1}<p class="notice">
      El historial disponible comienza en la versi&#243;n {firstAvailableVersion}.
    </p>{/if}
  <div class="version-list">
    {#each versions as version}<button
        class="version-row"
        class:selected={selectedVersion === version.version}
        aria-pressed={selectedVersion === version.version}
        onclick={() => onselect(version)}
      >
        <span class="file-icon"><Icon name="file" size={22} /></span><span class="version-text"
          ><strong>Versi&#243;n {version.version} / {version.name}</strong><small
            >{version.sealed ? 'Sellada' : 'Pendiente de sello'}</small
          ></span
        ><span class="badge" class:info={version.version === currentVersion}
          >{version.version === currentVersion ? 'Actual' : 'Hist\u00f3rica'}</span
        >
      </button>{/each}
  </div>
  {#if busy}<p class="hint" role="status">Consultando versiones...</p>{/if}
  {#if hasMore}<div class="action-row">
      <button class="secondary" disabled={busy} onclick={onmore}>Cargar versiones anteriores</button
      >
    </div>{/if}
</section>
