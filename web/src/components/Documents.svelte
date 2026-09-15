<script>
  import { onMount, onDestroy, tick } from 'svelte';
  import Icon from './Icon.svelte';
  import DocumentVersions from './DocumentVersions.svelte';
  import DocumentList from './DocumentList.svelte';
  import UploadDocument from './UploadDocument.svelte';
  import Pagination from './Pagination.svelte';
  import { can, validId } from '../lib/documents.mjs';
  export let api;
  export let user;
  export let caseRecord;
  export let intent = null;
  export let onintent = () => {};
  const scoped = api.caseDocuments(caseRecord.id);
  let documents = [];
  let selected = null;
  let upload;
  let reference = '';
  let search = '';
  let name = '';
  let filter = 'all';
  let layout = 'list';
  let offset = 0;
  let hasMore = false;
  let busy = false;
  let opening = false;
  let error = '';
  let detail;
  let alive = true;
  let listGeneration = 0;
  let detailGeneration = 0;
  function invalidateDetail() {
    detailGeneration++;
    opening = false;
  }
  async function load(nextOffset = 0) {
    const current = ++listGeneration;
    busy = true;
    error = '';
    documents = [];
    hasMore = false;
    offset = nextOffset;
    try {
      const result = await scoped.list({
        limit: 50,
        offset,
        name,
        sealed: filter === 'all' ? undefined : filter === 'sealed',
      });
      if (!alive || current !== listGeneration) return;
      documents = result.documents;
      hasMore = result.has_more;
    } catch (failure) {
      if (alive && current === listGeneration) {
        selected = null;
        invalidateDetail();
        error = failure.message;
      }
    } finally {
      if (alive && current === listGeneration) busy = false;
    }
  }
  async function focus() {
    await tick();
    if (!alive) return;
    detail?.scrollIntoView({ block: 'start' });
    detail?.focus({ preventScroll: true });
  }
  async function openDocument(document) {
    const current = ++detailGeneration;
    selected = null;
    opening = true;
    error = '';
    try {
      const result = await scoped.detail(document.id);
      if (!alive || current !== detailGeneration) return;
      selected = result;
      focus();
    } catch (failure) {
      if (alive && current === detailGeneration) error = failure.message;
    } finally {
      if (alive && current === detailGeneration) opening = false;
    }
  }
  function openReference(event) {
    event.preventDefault();
    const id = reference.trim().toLowerCase();
    if (!validId(id)) {
      error = 'Ingresa un identificador UUID v\u00e1lido.';
      return;
    }
    openDocument({ id });
  }
  function update(document) {
    if (!alive || selected?.id !== document.id) return;
    const changed = selected.sealed !== document.sealed || selected.version !== document.version;
    selected = document;
    if (changed) load(offset);
  }
  function uploaded(document) {
    if (!alive) return;
    invalidateDetail();
    selected = document;
    load(0);
    focus();
  }
  function applySearch(event) {
    event.preventDefault();
    name = search.trim();
    selected = null;
    invalidateDetail();
    load(0);
  }
  onMount(() => {
    if (can(user.role, 'documents')) {
      filter = ['pending', 'sealed'].includes(intent?.filter) ? intent.filter : 'all';
      load();
      if (intent?.type === 'upload') upload.open();
      if (intent?.id) openDocument({ id: intent.id });
    }
    onintent();
  });
  onDestroy(() => {
    alive = false;
    listGeneration++;
    detailGeneration++;
    scoped.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">ARCHIVO DEL EXPEDIENTE</span>
    <h1>Documentos</h1>
    <p>Del archivo original a una evidencia que puedes verificar.</p>
  </div>
  {#if can(user.role, 'documents')}<button class="primary" onclick={() => upload.open()}
      ><Icon name="plus" size={18} />Subir documento</button
    >{/if}
</div>
{#if !can(user.role, 'documents')}<section class="empty-state card">
    <Icon name="lock" size={40} />
    <h2>Acceso documental pendiente</h2>
    <p>
      Tu cuenta permite consultar los datos del expediente. El acceso a documentos para clientes
      a&#250;n no est&#225; habilitado.
    </p>
  </section>
{:else}
  <UploadDocument bind:this={upload} api={scoped} onuploaded={uploaded} />
  <section class="card document-list" aria-busy={busy}>
    <div class="section-heading">
      <div>
        <h2>Archivo documental</h2>
        <p class="hint">
          Documentos guardados en {caseRecord.reference}. B&#250;squeda por nombre en el expediente.
        </p>
      </div>
      <button class="secondary" disabled={busy} onclick={() => load(offset)}>Actualizar</button>
    </div>
    <div class="list-toolbar">
      <form class="server-search" onsubmit={applySearch}>
        <label class="search-field"
          ><Icon name="search" size={18} /><input
            aria-label="Buscar por nombre"
            placeholder="Buscar por nombre..."
            bind:value={search}
          /></label
        ><button class="secondary">Buscar</button>
      </form>
      <select
        aria-label="Filtrar por estado"
        bind:value={filter}
        onchange={() => {
          selected = null;
          invalidateDetail();
          load(0);
        }}
        ><option value="all">Todos los estados</option><option value="pending"
          >Pendientes de sello</option
        ><option value="sealed">Sellados</option></select
      >
      <div class="view-switch" aria-label="Presentaci&#243;n del listado">
        <button
          class:active={layout === 'list'}
          class="icon-button"
          aria-label="Vista de lista"
          aria-pressed={layout === 'list'}
          onclick={() => (layout = 'list')}><Icon name="list" size={18} /></button
        ><button
          class:active={layout === 'grid'}
          class="icon-button"
          aria-label="Vista de tarjetas"
          aria-pressed={layout === 'grid'}
          onclick={() => (layout = 'grid')}><Icon name="grid" size={18} /></button
        >
      </div>
    </div>
    {#if busy}<p class="hint" role="status">Consultando documentos...</p>
    {:else if documents.length}<DocumentList
        {documents}
        {layout}
        selectedId={selected?.id}
        onselect={openDocument}
      />
    {:else if !error}<div class="empty-state">
        <span class="empty-icon"><Icon name="folder" size={35} /></span>
        <h3>
          {name || filter !== 'all'
            ? 'No encontramos coincidencias'
            : 'Tu archivo empieza con el primer documento'}
        </h3>
        <p>
          {name || filter !== 'all'
            ? 'Cambia la b\u00fasqueda o el estado para ver otros documentos.'
            : 'Carga un archivo para incorporarlo a este expediente.'}
        </p>
        {#if name || filter !== 'all'}<button
            class="secondary"
            onclick={() => {
              search = '';
              name = '';
              filter = 'all';
              load(0);
            }}>Limpiar filtros</button
          >{:else}<button class="secondary" onclick={() => upload.open()}
            >Seleccionar un archivo</button
          >{/if}
      </div>{/if}
    <Pagination {offset} count={documents.length} {hasMore} {busy} onchange={load} />
  </section>
  <section class="card reference-panel">
    <div class="reference-intro">
      <span class="tile-icon"><Icon name="search" size={23} /></span>
      <div>
        <h2>Abre un documento por identificador</h2>
        <p>Consulta un archivo de este expediente con su identificador.</p>
      </div>
    </div>
    <form onsubmit={openReference}>
      <label
        >Identificador del documento<input
          required
          placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
          bind:value={reference}
        /></label
      ><button class="secondary" disabled={opening}
        >{opening ? 'Consultando...' : 'Abrir documento'}<Icon name="arrow" size={16} /></button
      >
    </form>
    <p class="hint">
      Abrir muestra sus datos. Usa Verificar integridad para comprobar la evidencia.
    </p>
  </section>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if selected}<div class="document-focus" tabindex="-1" bind:this={detail}>
      {#key selected.id}<DocumentVersions
          api={scoped}
          {user}
          document={selected}
          onupdate={update}
        />{/key}
    </div>{/if}
{/if}
