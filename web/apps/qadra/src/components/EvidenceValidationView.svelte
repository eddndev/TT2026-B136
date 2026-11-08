<script>
  import EvidenceValidation from './EvidenceValidation.svelte';
  export let api;
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">Evidencia</span>
    <h1>Validar evidencia</h1>
    <p>
      Revisa en este navegador un paquete que descargaste: el acuse parcial de tu firma o el
      contrato firmado por ambas partes.
    </p>
  </div>
</div>
<section class="card">
  <EvidenceValidation {api} />
</section>
