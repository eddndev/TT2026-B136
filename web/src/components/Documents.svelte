<script>
  import Icon from './Icon.svelte';
  import DocumentDetail from './DocumentDetail.svelte';
  import { can, validateUpload, validId, upsertDocument } from '../lib/documents.mjs';
  export let api;
  export let user;
  export let documents;
  export let ondocuments;
  let selected = null;
  let showUpload = false;
  let file = null;
  let name = '';
  let reference = '';
  let search = '';
  let filter = 'all';
  let busy = false;
  let error = '';
  $: visible = documents.filter(
    (item) =>
      `${item.name} ${item.id}`.toLowerCase().includes(search.toLowerCase()) &&
      (filter === 'all' || (filter === 'sealed' ? item.sealed : !item.sealed)),
  );
  function update(document) {
    ondocuments(upsertDocument(documents, document));
    selected = document;
  }
  async function upload(event) {
    event.preventDefault();
    error = validateUpload(file, name);
    if (error) return;
    busy = true;
    try {
      update(await api.upload(file, name.trim()));
      showUpload = false;
      file = null;
      name = '';
    } catch (failure) {
      error = failure.message;
    } finally {
      busy = false;
    }
  }
  async function openReference(event) {
    event.preventDefault();
    error = '';
    const id = reference.trim();
    if (!validId(id)) {
      error = 'Ingresa un identificador UUID valido.';
      return;
    }
    busy = true;
    try {
      const report = await api.verify(id);
      const known = documents.find((item) => item.id === id);
      update({
        ...known,
        id,
        name: known?.name || 'Documento por identificador',
        digest: report.document_digest,
        report,
      });
    } catch (failure) {
      error = failure.message;
    } finally {
      busy = false;
    }
  }
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">ESPACIO DOCUMENTAL</span>
    <h1>Tu mesa de trabajo</h1>
    <p>Cada documento, con el respaldo que necesita.</p>
  </div>
  {#if can(user.role, 'documents')}<button
      class="primary"
      onclick={() => {
        showUpload = !showUpload;
        error = '';
      }}><Icon name="upload" size={18} />Subir documento</button
    >{/if}
</div>
{#if !can(user.role, 'documents')}
  <section class="empty-state card">
    <Icon name="lock" size={40} />
    <h2>Acceso documental pendiente</h2>
    <p>
      El acceso de clientes estara disponible cuando se habilite la asignacion de expedientes.
      Contacta al administrador del despacho.
    </p>
  </section>
{:else}
  <section class="workspace-banner">
    <div>
      <span class="eyebrow">UN ARCHIVO. TODA SU EVIDENCIA.</span>
      <h2>Del documento a la certeza.</h2>
      <p>Carga, sella y verifica sin salir de tu espacio.</p>
    </div>
    <div class="process-steps">
      <span><Icon name="upload" />Cargar</span><i></i><span><Icon name="lock" />Sellar</span><i
      ></i><span><Icon name="shield" />Verificar</span>
    </div>
  </section>
  {#if showUpload}
    <section class="card upload-panel">
      <div class="section-heading">
        <h2>Nuevo documento</h2>
        <button
          class="icon-button"
          aria-label="Cerrar carga"
          disabled={busy}
          onclick={() => (showUpload = false)}><Icon name="close" /></button
        >
      </div>
      <form class="stack" onsubmit={upload}>
        <label class="file-drop"
          ><Icon name="upload" size={30} /><strong>Selecciona el archivo de tu equipo</strong><span
            >Cualquier formato, hasta 16 MiB.</span
          ><span class="sr-only">Archivo</span><input
            aria-label="Archivo"
            type="file"
            required
            onchange={(event) => {
              file = event.currentTarget.files[0] || null;
              name = file?.name || '';
            }}
          /></label
        ><label
          >Nombre del documento<input
            required
            bind:value={name}
            placeholder="contrato.pdf"
          /></label
        ><small>El servidor recibe el nombre en una cabecera: utiliza letras sin acentos.</small
        ><button class="primary" disabled={busy}>{busy ? 'Cargando...' : 'Cargar documento'}</button
        >
      </form>
    </section>
  {/if}
  <div class="document-layout">
    <section class="card document-list">
      <div class="section-heading">
        <h2>Documentos de esta sesion <span class="count">{documents.length}</span></h2>
      </div>
      <p class="hint">
        Referencias abiertas o cargadas aqui. Se limpian al salir o recargar; los archivos siguen en
        el servidor.
      </p>
      <div class="list-toolbar">
        <label class="search-field"
          ><Icon name="search" size={17} /><input
            aria-label="Buscar en esta sesion"
            placeholder="Buscar en esta sesion"
            bind:value={search}
          /></label
        ><select aria-label="Filtrar por estado" bind:value={filter}
          ><option value="all">Todos los estados</option><option value="sealed">Sellados</option
          ><option value="pending">Sin confirmar sello</option></select
        >
      </div>
      {#if visible.length}
        <div class="document-rows">
          {#each visible as item}<button
              class="document-row"
              class:selected={selected?.id === item.id}
              onclick={() => {
                selected = item;
              }}
              ><span class="file-icon"><Icon name="file" /></span><span class="document-name"
                ><strong>{item.name}</strong><small
                  >{item.id.slice(0, 8)} / {item.version ? `v${item.version}` : 'Referencia'}</small
                ></span
              ><span class="badge" class:success={item.sealed}
                >{item.sealed ? 'Sellado' : 'Por verificar'}</span
              ><Icon name="arrow" size={16} /></button
            >{/each}
        </div>
      {:else}<div class="empty-state">
          <span class="empty-icon"><Icon name="folder" size={32} /></span>
          <h3>{documents.length ? 'Sin coincidencias' : 'Tu siguiente documento empieza aqui'}</h3>
          <p>
            {documents.length
              ? 'Prueba otro nombre o estado.'
              : 'Carga un archivo o abre uno existente con su identificador.'}
          </p>
        </div>{/if}
    </section>
    <aside class="card reference-panel">
      <span class="tile-icon"><Icon name="search" size={23} /></span>
      <h2>Ya tienes un documento?</h2>
      <p>Usa su identificador para abrirlo y comprobar su integridad.</p>
      <form class="stack" onsubmit={openReference}>
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
      <p class="hint">La consulta genera una verificacion en la bitacora.</p>
    </aside>
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if selected}{#key selected.id}<DocumentDetail
        {api}
        {user}
        document={selected}
        onupdate={update}
      />{/key}{/if}
{/if}
