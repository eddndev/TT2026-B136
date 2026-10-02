# Material de práctica

Estos archivos contienen texto ASCII ficticio, sin personas ni asuntos reales:
[practica-v1.txt](practica-v1.txt) y [practica-v2.txt](practica-v2.txt).
Sus contenidos distintos permiten distinguir versiones; no son un contrato,
una actuación procesal ni un documento con validez jurídica.

La instancia corresponde a un solo despacho: Owner puede consultar todos sus
expedientes. Un prefijo de sesión no crea aislamiento. Usa una instancia de
práctica por sesión o restaura sólo el entorno desechable a su estado sintético
inicial entre sesiones, después de conservar la evidencia autorizada. Nunca
apliques esa preparación sobre una instalación con información real.

Antes de cada sesión, el moderador debe preparar y comprobar:

- Cuenta de práctica independiente del rol elegido y segundo factor configurado.
  Usa correo bajo `example.test`; conserva secretos fuera de las hojas y capturas.
- Expediente activo con título `Práctica Qadra Sxxx` y referencia `USO-Sxxx`, donde
  `Sxxx` se sustituye por el identificador asignado a esa sesión. Debe estar asignado
  al Litigante o Asistente que lo utilizará. Todos sus datos son ficticios.
- Otro expediente visible con título distinto para observar selección consciente.
  El expediente de otra sesión no sirve como distractor: debe quedar aislado.
- Ambos TXT disponibles en una carpeta que el participante conozca. T03 crea el
  documento inicial; T05 usa el segundo archivo como versión, no como alta nueva.
- Para T04-V, una copia del primer TXT llamada `verificacion-preparada.txt`,
  cargada y sellada previamente por una cuenta autorizada del entorno de práctica.
- Para T07, cuenta auxiliar ficticia existente, activa y no asignada inicialmente.
  No se necesita iniciar sesión con ella durante la tarea.

Registra las identidades de prueba y estados en la hoja privada de preparación.
No agregues al repositorio un volcado de base de datos, credenciales ni resultados.
No copies un expediente real para cambiarle sólo los nombres.

Los módulos adicionales requieren preparación específica:

- T06 necesita audiencia y plazo vigentes confirmados mediante los flujos
  documentados en el [manual](../../user-guide.md). El moderador entrega el periodo,
  desfase, expediente y actividad que deben encontrarse. Si no puede preparar un
  escenario correcto con los catálogos ya implementados, excluye esa tarea.
- T09 necesita dos expedientes de práctica autorizados, uno activo y otro cerrado,
  cuyas fechas de creación estén dentro del intervalo UTC entregado.
- T10 necesita más de 20 eventos reales del entorno ficticio para un filtro exacto
  de recurso. Se generan mediante acciones autorizadas sobre esos datos; no se
  insertan filas para inventar una historia. La aceptación técnica del módulo
  comprueba la integridad; la persona sólo observa su presentación y consulta.

La comprobación de preparación se registra como ensayo técnico. No cuenta como
una sesión humana ni como finalización de tareas del participante.
