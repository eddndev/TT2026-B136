<script>
  import { onDestroy } from 'svelte';
  import MemberEnrollment from './MemberEnrollment.svelte';
  import MemberDirectory from './MemberDirectory.svelte';
  import Icon from './Icon.svelte';
  import AuditEvents from './AuditEvents.svelte';
  export let api;
  export let view;
  export let user;
  let audit = null;
  let busy = false;
  let error = '';
  let alive = true,
    directory,
    directoryRoot;
  async function submit(event) {
    event?.preventDefault();
    busy = true;
    error = '';
    try {
      audit = null;
      const value = await api.audit();
      if (alive) audit = value;
    } catch (failure) {
      if (alive) error = failure.message;
    } finally {
      busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">ADMINISTRACI&Oacute;N DEL DESPACHO</span>
    <h1>{view === 'users' ? 'Tu equipo, conectado' : 'Una historia verificable'}</h1>
    <p>
      {view === 'users'
        ? 'Crea accesos con los permisos adecuados.'
        : 'Comprueba la continuidad de la bit\u00e1cora del despacho.'}
    </p>
  </div>
</div>
{#if view === 'users'}
  <div tabindex="-1" bind:this={directoryRoot}>
    <MemberDirectory bind:this={directory} {api} {user} />
  </div>
  <MemberEnrollment
    {api}
    {user}
    oncomplete={() => directory?.refresh()}
    onreturn={() => directoryRoot?.focus()}
  />
{:else}
  <section class="card admin-panel">
    <span class="tile-icon"><Icon name="shield" size={28} /></span>
    <h2>Integridad de la bit&aacute;cora</h2>
    <p>
      Verifica la cadena completa de eventos registrados por el servidor y detecta el primer enlace
      roto.
    </p>
    <button class="primary" disabled={busy} onclick={submit}
      >{busy ? 'Verificando...' : 'Verificar cadena'}</button
    >
    {#if audit}<div
        class="notice"
        class:success={audit.valid}
        class:error={!audit.valid}
        role="status"
      >
        <h3>{audit.valid ? 'Cadena \u00edntegra' : 'Alteraci\u00f3n detectada'}</h3>
        <p>
          {audit.valid
            ? audit.entries === 1
              ? '1 evento verificado'
              : `${audit.entries} eventos verificados`
            : `Primer \u00edndice roto: ${audit.first_broken_index}`}
        </p>
      </div>{/if}
    <p class="hint">
      La verificaci&oacute;n comprueba la cadena local; no certifica un anclaje externo de la
      bit&aacute;cora.
    </p>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  </section>
{/if}

{#if view === 'audit'}<AuditEvents {api} {user} />{/if}
