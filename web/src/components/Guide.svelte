<script>
  import Icon from './Icon.svelte';
  export let onnavigate;
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">AYUDA Y ORIENTACI&Oacute;N</span>
    <h1>Un recorrido claro, de principio a fin</h1>
    <p>Lo que necesitas para trabajar con tus documentos.</p>
  </div>
</div>
<section class="guide-grid">
  {#each [{ icon: 'upload', title: '1. Carga tu documento', text: 'Selecciona o arrastra un archivo de hasta 16 MiB. Te sugerimos un nombre compatible; puedes revisarlo antes de cargarlo.' }, { icon: 'lock', title: '2. Genera su evidencia', text: 'Un administrador o litigante puede firmar y sellar. La confirmaci\u00f3n registra esta acci\u00f3n con la identidad de quien la realiza.' }, { icon: 'shield', title: '3. Comprueba el resultado', text: 'Revisa la integridad, la firma, el certificado y el sello. Si hay alg\u00fan fallo, el resultado te indicar\u00e1 qu\u00e9 componente necesita revisi\u00f3n.' }, { icon: 'download', title: '4. Conserva el respaldo', text: 'Descarga el paquete ZIP y guarda el identificador del documento para volver a abrirlo, incluso despu\u00e9s de cerrar sesi\u00f3n.' }] as item}<article
      class="card"
    >
      <span class="tile-icon"><Icon name={item.icon} size={24} /></span>
      <h2>{item.title}</h2>
      <p>{item.text}</p>
    </article>{/each}
</section>
<section class="card guide-questions">
  <h2>Antes de continuar</h2>
  <details>
    <summary>&iquest;D&oacute;nde est&aacute;n mis documentos anteriores?</summary>
    <p>
      Los archivos permanecen en el servidor. La lista de trabajo solo muestra los abiertos en tu
      sesi&oacute;n actual: usa el identificador que recibiste al cargar cada archivo para volver a
      abrirlo.
    </p>
  </details>
  <details>
    <summary>&iquest;Qui&eacute;n puede hacer cada cosa?</summary>
    <p>
      El administrador y el litigante pueden cargar, sellar, verificar y descargar. El asistente
      legal puede cargar, verificar y descargar; el sellado requiere un administrador o litigante.
      La auditor&iacute;a completa y el alta de usuarios corresponden al administrador.
    </p>
  </details>
  <details>
    <summary>&iquest;Qu&eacute; significa sellado local?</summary>
    <p>
      Es una demostraci&oacute;n t&eacute;cnica de firma y sello de tiempo. No equivale a una
      constancia NOM-151 emitida por un PSC autorizado.
    </p>
  </details>
  <details>
    <summary>&iquest;Puedo organizar expedientes y audiencias?</summary>
    <p>
      Esas funciones a&uacute;n no est&aacute;n disponibles en esta interfaz. Hoy puedes completar
      el flujo documental y conservar la evidencia de cada archivo.
    </p>
  </details>
  <details>
    <summary>&iquest;Qu&eacute; pasa si pierdo mi segundo factor?</summary>
    <p>
      Usa uno de los c&oacute;digos de recuperaci&oacute;n guardados durante el alta. Cada
      c&oacute;digo funciona una sola vez. Si no conservas ninguno, contacta al administrador.
    </p>
  </details>
  <button class="primary" onclick={() => onnavigate('documents')}
    >Ir a documentos<Icon name="arrow" size={16} /></button
  >
</section>
