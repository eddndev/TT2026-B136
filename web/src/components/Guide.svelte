<script>
  import Icon from './Icon.svelte';
  export let onnavigate;
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">AYUDA Y ORIENTACION</span>
    <h1>Un recorrido claro, de principio a fin</h1>
    <p>Lo que necesitas para trabajar con tus documentos.</p>
  </div>
</div>
<section class="guide-grid">
  {#each [{ icon: 'upload', title: '1. Carga tu documento', text: 'Selecciona o arrastra un archivo de hasta 16 MiB. Te sugerimos un nombre compatible; puedes revisarlo antes de cargarlo.' }, { icon: 'lock', title: '2. Genera su evidencia', text: 'Un administrador o litigante puede firmar y sellar. La confirmacion registra esta accion con la identidad de quien la realiza.' }, { icon: 'shield', title: '3. Comprueba el resultado', text: 'Revisa la integridad, firma, certificado y sello. Si hay algun fallo, el resultado te indicara que componente necesita revision.' }, { icon: 'download', title: '4. Conserva el respaldo', text: 'Descarga el paquete ZIP y guarda el identificador del documento para volver a abrirlo, incluso despues de cerrar sesion.' }] as item}<article
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
    <summary>Donde estan mis documentos anteriores?</summary>
    <p>
      Los archivos permanecen en el servidor. La lista de trabajo solo muestra los abiertos en tu
      sesion actual: usa el identificador que recibiste al cargar para volver a abrirlos.
    </p>
  </details>
  <details>
    <summary>Quien puede hacer cada cosa?</summary>
    <p>
      Administrador y litigante pueden cargar, sellar, verificar y descargar. El asistente legal
      puede cargar, verificar y descargar; el sellado requiere un administrador o litigante. La
      auditoria completa y el alta de usuarios son del administrador.
    </p>
  </details>
  <details>
    <summary>Que significa sellado local?</summary>
    <p>
      Es una demostracion tecnica de firma y sello de tiempo. No equivale a una constancia NOM-151
      emitida por un PSC autorizado.
    </p>
  </details>
  <details>
    <summary>Puedo organizar expedientes y audiencias?</summary>
    <p>
      Esas funciones aun no estan disponibles en esta interfaz. Hoy puedes completar el flujo
      documental y conservar la evidencia de cada archivo.
    </p>
  </details>
  <details>
    <summary>Que pasa si pierdo mi segundo factor?</summary>
    <p>
      Usa uno de los codigos de recuperacion guardados durante el alta. Cada codigo funciona una
      sola vez. Si no conservas ninguno, contacta al administrador.
    </p>
  </details>
  <button class="primary" onclick={() => onnavigate('documents')}
    >Ir a documentos<Icon name="arrow" size={16} /></button
  >
</section>
