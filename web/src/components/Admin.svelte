<script>
  import Enrollment from './Enrollment.svelte';
  import Icon from './Icon.svelte';
  export let api;
  export let view;
  let email = '';
  let password = '';
  let role = 'paralegal';
  let enrollment = null;
  let audit = null;
  let busy = false;
  let error = '';
  async function submit(event) {
    event?.preventDefault();
    busy = true;
    error = '';
    try {
      if (view === 'users') enrollment = await api.createUser(email.trim(), password, role);
      else {
        audit = null;
        audit = await api.audit();
      }
    } catch (failure) {
      error = failure.message;
    } finally {
      password = '';
      busy = false;
    }
  }
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">ADMINISTRACION DEL DESPACHO</span>
    <h1>{view === 'users' ? 'Tu equipo, conectado' : 'Una historia verificable'}</h1>
    <p>
      {view === 'users'
        ? 'Crea accesos con los permisos adecuados.'
        : 'Comprueba la continuidad de la bitacora del despacho.'}
    </p>
  </div>
</div>
<section class="card admin-panel">
  {#if view === 'users'}
    {#if enrollment}<Enrollment
        {enrollment}
        ondone={() => {
          enrollment = null;
          email = '';
        }}
      />
    {:else}<span class="tile-icon"><Icon name="users" size={26} /></span>
      <h2>Nuevo integrante</h2>
      <p>
        El material de segundo factor se mostrara una sola vez. Entregalo de forma segura a su
        titular.
      </p>
      <form class="stack" onsubmit={submit}>
        <label
          >Correo del nuevo usuario<input
            type="email"
            required
            autocomplete="off"
            bind:value={email}
          /></label
        ><label
          >Contrasena temporal<input
            type="password"
            minlength="12"
            required
            autocomplete="new-password"
            bind:value={password}
          /></label
        ><small
          >Minimo 12 caracteres. La API actual no ofrece cambio de contrasena desde la interfaz.</small
        ><label
          >Rol<select bind:value={role}
            ><option value="paralegal">Asistente legal</option><option value="litigator"
              >Litigante</option
            ><option value="owner">Administrador</option><option value="client">Cliente</option
            ></select
          ></label
        >
        <p class="notice">
          {role === 'owner'
            ? 'Acceso a documentos, sellado, auditoria y alta de usuarios.'
            : role === 'litigator'
              ? 'Puede cargar, sellar, verificar y exportar documentos.'
              : role === 'paralegal'
                ? 'Puede cargar, verificar y exportar documentos. No puede sellar.'
                : 'Sin acceso documental hasta contar con asignacion a expedientes.'}
        </p>
        <button class="primary" disabled={busy}>{busy ? 'Creando...' : 'Crear usuario'}</button>
      </form>{/if}
  {:else}<span class="tile-icon"><Icon name="shield" size={28} /></span>
    <h2>Integridad de la bitacora</h2>
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
        <h3>{audit.valid ? 'Cadena integra' : 'Alteracion detectada'}</h3>
        <p>
          {audit.valid
            ? `${audit.entries} eventos verificados`
            : `Primer indice roto: ${audit.first_broken_index}`}
        </p>
      </div>{/if}
    <p class="hint">
      La verificacion comprueba la cadena local; no certifica un anclaje externo de la bitacora.
    </p>
  {/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</section>
