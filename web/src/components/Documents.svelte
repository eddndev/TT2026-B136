<script>
  import { onMount, tick } from 'svelte';
  import Icon from './Icon.svelte';
  import DocumentDetail from './DocumentDetail.svelte';
  import DocumentList from './DocumentList.svelte';
  import UploadDocument from './UploadDocument.svelte';
  import { can, validId, upsertDocument } from '../lib/documents.mjs';
  import { filterDocuments } from '../lib/workspace.mjs';
  export let api;
  export let user;
  export let documents;
  export let ondocuments;
  export let intent = null;
  export let onintent = () => {};
  let selected = null;
  let upload;
  let reference = '';
  let search = '';
  let filter = 'all';
  let layout = 'list';
  let busy = false;
  let error = '';
  let detail;
  $: visible = filterDocuments(documents, search, filter);
  function update(document) {
    ondocuments(upsertDocument(documents, document));
    selected = document;
  }
  async function focusDocument(document) {
    selected = document;
    await tick();
    detail?.scrollIntoView({ block: 'start' });
    detail?.focus({ preventScroll: true });
  }
  async function openReference(event) {
    event.preventDefault();
    error = '';
    const id = reference.trim().toLowerCase();
    if (!validId(id)) {
      error = 'Ingresa un identificador UUID valido.';
      return;
    }
    busy = true;
    const known = documents.find((item) => item.id === id);
    if (known) update({ ...known, report: null });
    try {
      const report = await api.verify(id);
      update({
        ...known,
        id,
        name: known?.name || 'Documento por identificador',
        digest: report.document_digest,
        sealed: true,
        report,
      });
      focusDocument(selected);
    } catch (failure) {
      if (failure.status === 409 && failure.code === 'document_not_sealed') {
        update({
          ...known,
          id,
          name: known?.name || 'Documento por identificador',
          sealed: false,
          report: null,
        });
        focusDocument(selected);
      } else error = failure.message;
    } finally {
      busy = false;
    }
  }
  onMount(() => {
    filter = intent?.filter || 'all';
    if (intent?.type === 'upload' && can(user.role, 'documents')) upload.open();
    if (intent?.id) {
      const match = documents.find((item) => item.id === intent.id);
      if (match) focusDocument(match);
    }
    onintent();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">ARCHIVO DEL DESPACHO</span>
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
      El acceso de clientes estara disponible cuando se habilite la asignacion de expedientes.
      Contacta al administrador del despacho.
    </p>
  </section>
{:else}
  <UploadDocument
    bind:this={upload}
    {api}
    onuploaded={(document) => {
      update(document);
      focusDocument(document);
    }}
  />
  <section class="card document-list">
    <div class="section-heading">
      <div>
        <h2>Mi mesa documental <span class="count">{documents.length}</span></h2>
        <p class="hint">
          Documentos abiertos en esta sesion. Conserva sus identificadores antes de salir.
        </p>
      </div>
      <span class="badge neutral">Sesion actual</span>
    </div>
    <div class="list-toolbar">
      <label class="search-field"
        ><Icon name="search" size={18} /><input
          aria-label="Buscar en esta sesion"
          placeholder="Buscar por nombre o identificador..."
          bind:value={search}
        /></label
      ><select aria-label="Filtrar por estado" bind:value={filter}
        ><option value="all">Todos los estados</option><option value="pending"
          >Pendientes de sello</option
        ><option value="sealed">Sellados</option><option value="verified">Verificados</option
        ><option value="failed">Revisar evidencia</option></select
      >
      <div class="view-switch" aria-label="Presentacion del listado">
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
    {#if visible.length}<DocumentList
        documents={visible}
        {layout}
        selectedId={selected?.id}
        onselect={focusDocument}
      />{:else}<div class="empty-state">
        <span class="empty-icon"><Icon name="folder" size={35} /></span>
        <h3>
          {documents.length
            ? 'No encontramos coincidencias'
            : 'Tu archivo empieza con el primer documento'}
        </h3>
        <p>
          {documents.length
            ? 'Cambia la busqueda o el estado para ver otros documentos.'
            : 'Carga un archivo nuevo o recupera uno existente con su identificador.'}
        </p>
        {#if documents.length}<button
            class="secondary"
            onclick={() => {
              search = '';
              filter = 'all';
            }}>Limpiar filtros</button
          >{:else}<button class="secondary" onclick={() => upload.open()}
            >Seleccionar un archivo</button
          >{/if}
      </div>{/if}
    <div class="list-footer">
      <span>{visible.length} de {documents.length} documentos</span><span
        >Los archivos permanecen guardados al cerrar sesion.</span
      >
    </div>
  </section>
  <section class="card reference-panel">
    <div class="reference-intro">
      <span class="tile-icon"><Icon name="search" size={23} /></span>
      <div>
        <h2>Recupera un documento anterior</h2>
        <p>Abre un archivo del servidor con el identificador que recibiste al cargarlo.</p>
      </div>
    </div>
    <form onsubmit={openReference}>
      <label
        >Identificador del documento<input
          required
          placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
          bind:value={reference}
        /></label
      ><button class="secondary" disabled={busy}
        >{busy ? 'Consultando...' : 'Abrir documento'}<Icon name="arrow" size={16} /></button
      >
    </form>
    <p class="hint">Al abrir un documento sellado tambien se comprueba su integridad.</p>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  </section>
  {#if selected}<div class="document-focus" tabindex="-1" bind:this={detail}>
      {#key selected.id}<DocumentDetail {api} {user} document={selected} onupdate={update} />{/key}
    </div>{/if}
{/if}
