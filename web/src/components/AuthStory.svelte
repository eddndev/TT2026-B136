<script>
  import Brand from './Brand.svelte';
  import Icon from './Icon.svelte';
</script>

<section class="auth-story">
  <a class="brand" href="/" aria-label="Qadra, inicio"><Brand /></a>
  <div class="story-copy">
    <span class="eyebrow">TU DESPACHO, EN ORDEN</span>
    <h1>Tu trabajo legal,<br /><span>bien resguardado.</span></h1>
    <p>
      Del primer archivo a su evidencia. Un espacio claro para cuidar cada documento de tu despacho.
    </p>
    <ul class="auth-benefits">
      <li>
        <span class="benefit-icon"><Icon name="upload" size={20} /></span>
        <div>
          <strong>Documentos bajo resguardo</strong>
          <p>Carga tus archivos y cons&eacute;rvalos cifrados.</p>
        </div>
      </li>
      <li>
        <span class="benefit-icon"><Icon name="shield" size={20} /></span>
        <div>
          <strong>Integridad verificable</strong>
          <p>Sella tus documentos y comprueba su integridad.</p>
        </div>
      </li>
      <li>
        <span class="benefit-icon"><Icon name="download" size={20} /></span>
        <div>
          <strong>Evidencia a la mano</strong>
          <p>Descarga el paquete de evidencia cuando lo necesites.</p>
        </div>
      </li>
    </ul>
  </div>
  <footer><span>Qadra / Evidencia documental</span><span>TT2026-B136</span></footer>
</section>
