<script>
  import Icon from './Icon.svelte';
  import { can } from '../lib/documents.mjs';
  import { sessionStats, documentStatus } from '../lib/workspace.mjs';
  export let user;
  export let documents;
  export let onnavigate;
  export let ondocument;
  $: stats = sessionStats(documents);
  $: pending = documents.filter((item) => item.sealed === false);
  const today = new Intl.DateTimeFormat('es-MX', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(new Date());
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">TU DESPACHO DIGITAL</span>
    <h1>Tu mesa de trabajo</h1>
    <p>Todo listo para continuar con tus documentos.</p>
  </div>
  <span class="today"><Icon name="calendar" size={16} />{today}</span>
</div>
{#if can(user.role, 'documents')}
  <section class="welcome-card">
    <div>
      <span class="eyebrow">MENOS PASOS. M&#193;S CLARIDAD.</span>
      <h2>Tu trabajo, en un solo lugar.</h2>
      <p>Carga un documento, conserva su evidencia y comprueba su integridad.</p>
      <div class="action-row">
        <button class="primary" onclick={() => ondocument({ type: 'upload' })}
          ><Icon name="plus" size={18} />Subir documento</button
        ><button class="text-button" onclick={() => onnavigate('guide')}
          >C&#243;mo funciona<Icon name="arrow" size={16} /></button
        >
      </div>
    </div>
    <div class="welcome-symbol" aria-hidden="true">
      <Icon name="folder" size={84} /><span><Icon name="shield" size={32} /></span>
    </div>
  </section>
  <div class="section-heading session-heading">
    <h2>Resumen de tu sesi&#243;n</h2>
    <span class="hint">Solo documentos abiertos en este acceso</span>
  </div>
  <section class="stats-grid" aria-label="Resumen de esta sesi&#243;n">
    <button class="stat-card" onclick={() => ondocument({ filter: 'all' })}
      ><div>
        <span>Documentos abiertos</span><strong>{stats.total}</strong><small
          >Disponibles en tu mesa</small
        >
      </div>
      <span class="stat-icon"><Icon name="file" size={23} /></span></button
    >
    <button class="stat-card" onclick={() => ondocument({ filter: 'pending' })}
      ><div>
        <span>Pendientes de sello</span><strong>{stats.pending}</strong><small
          >Siguiente paso del documento</small
        >
      </div>
      <span class="stat-icon warning"><Icon name="clock" size={23} /></span></button
    >
    <button class="stat-card" onclick={() => ondocument({ filter: 'verified' })}
      ><div>
        <span>Verificados</span><strong>{stats.verified}</strong><small
          >Con resultado v&#225;lido</small
        >
      </div>
      <span class="stat-icon success"><Icon name="shield" size={23} /></span></button
    >
  </section>
  <div class="overview-grid">
    <section class="card recent-panel">
      <div class="section-heading">
        <h2>Documentos recientes</h2>
        <button class="text-button" onclick={() => onnavigate('documents')}
          >Ver todos<Icon name="arrow" size={15} /></button
        >
      </div>
      {#if documents.length}<div class="recent-list">
          {#each documents.slice(0, 4) as item}{@const state = documentStatus(item)}<button
              class="recent-row"
              onclick={() => ondocument({ id: item.id })}
              ><span class="file-icon"><Icon name="file" /></span><span class="document-name"
                ><strong>{item.name}</strong><small
                  >{item.id.slice(0, 8)} / {item.version
                    ? `Versi\u00f3n ${item.version}`
                    : 'Referencia'}</small
                ></span
              ><span class="badge {state.tone}">{state.label}</span><Icon
                name="arrow"
                size={16}
              /></button
            >{/each}
        </div>
      {:else}<div class="empty-state">
          <span class="empty-icon"><Icon name="folder" size={33} /></span>
          <h3>Aqu&#237; empieza tu trabajo</h3>
          <p>Los documentos que abras o cargues aparecer&#225;n aqu&#237;.</p>
          <button class="secondary" onclick={() => ondocument({ type: 'upload' })}
            >Cargar mi primer documento</button
          >
        </div>{/if}
      <p class="panel-footnote">
        Al salir se limpia esta lista. Tus documentos permanecen en el servidor.
      </p>
    </section>
    <aside class="card next-panel">
      <span class="tile-icon"><Icon name="checklist" size={23} /></span>
      <h2>Tu siguiente paso</h2>
      {#if pending.length}<p>
          Tienes {pending.length}
          {pending.length === 1 ? 'documento sin sellar' : 'documentos sin sellar'} en esta sesi&#243;n.
        </p>
        <button class="secondary" onclick={() => ondocument({ id: pending[0].id })}
          >Revisar documento<Icon name="arrow" size={16} /></button
        >{:else}<p>Cada archivo sigue un recorrido sencillo.</p>{/if}
      <ol class="mini-steps">
        <li>
          <span>1</span>
          <div><strong>Carga</strong><small>El archivo se guarda cifrado.</small></div>
        </li>
        <li>
          <span>2</span>
          <div><strong>Sella</strong><small>Genera la firma y el sello local.</small></div>
        </li>
        <li>
          <span>3</span>
          <div>
            <strong>Verifica y descarga</strong><small>Revisa y conserva tu evidencia.</small>
          </div>
        </li>
      </ol>
    </aside>
  </div>
{:else}<section class="card empty-state">
    <Icon name="lock" size={40} />
    <h2>Tu acceso est&#225; listo</h2>
    <p>
      Tu cuenta de cliente a&#250;n no tiene acceso documental. El administrador podr&#225;
      orientarte sobre la entrega de documentos.
    </p>
    <button class="secondary" onclick={() => onnavigate('guide')}>Consultar gu&#237;a de uso</button
    >
  </section>{/if}
