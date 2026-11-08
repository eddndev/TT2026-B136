<script>
  import MemberEnrollment from './MemberEnrollment.svelte';
  import MemberInvitations from './MemberInvitations.svelte';
  import MemberDirectory from './MemberDirectory.svelte';
  export let api;
  export let user;
  let directory, directoryRoot;
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">Administraci&oacute;n del despacho</span>
    <h1>Tu equipo, conectado</h1>
    <p>Crea accesos con los permisos adecuados.</p>
  </div>
</div>
<div tabindex="-1" bind:this={directoryRoot}>
  <MemberDirectory bind:this={directory} {api} {user} />
</div>
<MemberInvitations {api} {user} />
<MemberEnrollment
  {api}
  {user}
  oncomplete={() => directory?.refresh()}
  onreturn={() => directoryRoot?.focus()}
/>
