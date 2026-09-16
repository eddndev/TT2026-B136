<script>
  import FactCollection from './FactCollection.svelte';
  import '../styles/procedural-facts.css';
  export let api, user, record, ondenied;
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">GESTI&#211;N DEL EXPEDIENTE</span>
    <h1>Resoluciones y notificaciones</h1>
    <p>Conserva cada declaraci&#243;n, sus fuentes y sus revisiones.</p>
  </div>
</div>
<FactCollection {api} {user} caseId={record.id} family="resolution" {ondenied} />
