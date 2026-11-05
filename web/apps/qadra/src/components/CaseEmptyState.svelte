<script>
  import Icon from '@qadra/ui/Icon.svelte';
  export let onnavigate;
</script>

<section class="card empty-state">
  <span class="empty-icon"><Icon name="briefcase" size={35} /></span>
  <h1>Selecciona un expediente</h1>
  <p>Abre un expediente para consultar sus datos.</p>
  <button class="primary" onclick={() => onnavigate('cases')}>Ver expedientes</button>
</section>
