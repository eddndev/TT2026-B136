# Cierre funcional del prototipo web

## Checkpoint funcional reconciliado

Estado conciliado contra el código y los resultados disponibles; las mediciones
históricas conservan el alcance de cada campaña.
PR45 y PR46 están integradas; PR47 quedó integrada como `ac34b34` y su
confirmación natural en `main` está comprobada. PR48 quedó integrada en
`5020707`; CI, Web y Documents de su confirmación natural aprobaron en 6m41s,
11m24s y 7m43s: 3156 pruebas Rust, 396 de navegador controlado, 47 reales y
dos ignoradas. Los informes se integraron por PR49 como `f3bfed8`, después de
aprobar CI, Web y Documents sobre la cabeza exacta `6e1ae2b`. Su confirmación
natural en `main` aprobó CI en 8m36s, Web en 11m46s y Documents en 1m13s,
con las mismas 3299 pruebas Rust, 409 controladas y 49 reales. La adaptación de PR43 a VPS3 está
integrada por squash en `3a67a6a`, con CI, Web y Documents aprobados para
`37087b8` y ambos coautores humanos. Su confirmación natural en main aprobó
CI en 6m42s, Web en 11m25s y Documents en 7m43s. La primera release privada `v0.1.0` quedó activada en VPS3 sobre `a8dc3dd`;
la salud, identidad del paquete y PKI aprobaron. La cuenta Owner todavía no está
creada y la aceptación autenticada permanece pendiente.
PR51 integró el respaldo Redis y la corrección del entorno CI como `673b9ec`
el 2 de octubre a las 10:42 UTC. Su cabeza `177fe1f` aprobó CI en 7m57s y
Documents en 1m18s. La confirmación natural de `673b9ece` aprobó CI
`36996915839` en 6m52s y Documents `36996915909` en 7m55s: 3340 pruebas Rust,
dos ignoradas, una nativa Redis y cobertura 97/95/93 %. Los controladores de
esa revisión se instalaron en VPS3. Posteriormente se aceptó `v0.1.1` sobre
`e3aa87a`, incluido rollback real a `v0.1.0` y retorno sin cambiar SQL ni claves.
Los nueve controladores de PR54/main `2de3326` ya están instalados, con CI y
Documents confirmados. La renovación real de CRL avanzó confianza 1 a 2,
preservó CA/claves/revocaciones y confirmó respaldo, reinicio y auditoría.
La aceptación autenticada sigue pendiente por falta
del correo de la primera cuenta Owner.

PR66–72 están integradas y su cierre posterior en `main` está confirmado:
recuperación de recursos/asociaciones, captura de restauración y su corrección
de observación, plazos contextuales, dominio del vínculo Owner y plazos ordinarios.
PR72 corrigió la espera del historial de audiencias tras el fallo natural de
PR71; `8db60f8` confirmó CI en 662 s y Web en 985 s, con 3532 pruebas Rust,
556 controladas y 52 reales. PR73 integró el verificador del vínculo Owner como
`6a574373b801dd6e40b8c459a35bc9461391444a` el 3 de octubre a las 11:32:33 UTC.
Su cabeza `1f6a0bb` aprobó CI en 751 s, Web en 1076 s y Documents en 80 s:
3540 pruebas Rust, 556 controladas y 52 reales, con cobertura 97/95/93 %.
La confirmación natural de `6a574373` aprobó CI en 656 s, Web en 988 s
y Documents en 555 s, con el mismo inventario y cobertura. Documents incluye
la espera de runner; su compilación duró 63 s.

PR74 integró la recuperación de informes como `309a0a73` y PR75 la de
preferencias de alertas como `81e525a0`, ambas con confirmación natural aprobada.
El último inventario completo de esas integraciones contiene 3540 pruebas Rust,
572 de navegador controlado y 52 reales, con cobertura de
`domain/application/infrastructure` de 97/95/93 %.
PR76 integró la conciliación del manual de acceso como `c300fed`; su CI natural
aprobó en 438 s. PR77 integró confirmaciones administrativas y de sellado como
`2ca02f9`: su cabeza aprobó CI en 565 s, Web en 1055 s y Documents en 75 s,
con 3540 pruebas Rust, 591 controladas y 52 reales. Su confirmación natural
también aprobó CI en 648 s, Web en 1049 s y Documents en 547 s, con el mismo
inventario. Esta evidencia no modifica
la release privada ni activa la política de inactividad.

PR78 integró el servicio de aplicación y los puertos del vínculo Owner como
`f8122205`. Su confirmación natural aprobó CI en 567 s, Web en 1065 s
y Documents en 76 s, con 3555 pruebas Rust, 591 controladas y 52 reales.
La cobertura de `domain/application/infrastructure` fue 98/95/93 %.
PR82 integró la persistencia, el transporte HTTP y la composición; PR83, la
interfaz del vínculo, y PR84, el primer factor Owner por certificado con MFA.
Sus confirmaciones naturales de CI, Web y Documents están aprobadas. PR81
integró la lectura de avisos, también con confirmación natural aprobada.
PR85 integró la recuperación del alta de integrantes como `e3b1eec`, después de
aprobar CI, Web y Documents para su cabeza exacta.
Estas integraciones no cambian la release privada ni activan opciones en VPS3.

- Integrados: plazos persistentes y reevaluación durable, agenda conjunta,
  alertas, recursos y actos declarados, asociaciones a actividades existentes,
  contenido documental y avisos de integridad, directorio y administración de
  acceso de miembros, calibración de contraseñas, tablero operativo autorizado y
  creación contextual explícita de plazos y sus asociaciones.
- Integrada por PR48: consulta inversa de recursos desde audiencia o plazo y
  regreso a la actividad o bandeja de alertas, con capturas exactas, permisos y
  filtros. Su aceptación API/restauración y navegador real aprobaron. La
  confirmación natural de CI, Web y Documents también aprobó en `main`.
- Pendientes procesales: corpus jurídico aplicable, activación automática y
  durable de un plazo nuevo, audiencias propias de recursos y catálogo restante
  de audiencias. Las continuaciones declaradas y la creación contextual explícita
  de plazos ya están integradas; no equivalen a activación jurídica automática.
  El catálogo separado de alegatos de apelación y revocación escrita, y la
  preparación contra recursos/actos exactos, confirmación de la revisión y
  conciliación explícita de la creación original tienen implementación local
  con pruebas focales. El contrato transaccional aún carece de adaptador SQL:
  no crean una audiencia persistente ni la incorporan a agenda o alertas;
  el pendiente continúa abierto. Véase el
  [contrato de audiencias de recursos](resource-hearings.md).
- Pendientes de identidad y documentos: invitaciones y enrolamiento recuperable,
  activación operativa de recuperación de contraseña, inactividad y primer factor
  Owner por certificado, cierre de las familias restantes de reingreso,
  firma individual y política documental Client. La admisión general de formatos
  está integrada por PR47 en `ac34b34`, con aceptación API/restauración,
  navegador real y confirmación natural de `main` comprobados.
  El aislamiento de respuestas tardías de contraseña y MFA tiene aceptación
  local: 524 pruebas Node y ocho recorridos de navegador aprobados. Se integró
  por PR53 como `e3aa87a`; CI y Web de main confirmaron 9m02s y 10m56s, con
  3340 Rust, 419 controladas y 51 reales. Ese aislamiento de respuestas no
  completa el recorrido de inactividad ni los borradores. Está desplegado en
  `v0.1.1`, con identidad y salud confirmadas.
- Sesiones, backend integrado por PR55 como `e5c8363`: estado y actividad
  explícitos conservan `absolute_only` de 24 horas por defecto. La inactividad
  exige configuración expresa y su duración operativa sigue sin aprobarse.
  La cabeza de PR55 aprobó CI, Web y Documents, con 3381 identidades Rust,
  419 de navegador controlado y 51 reales. En la confirmación natural de main
  aprobaron CI y Documents; Web se canceló tras fallar una medición geométrica
  del historial móvil mientras se reemplazaba su fila. PR56 corrigió esa carrera:
  CI 7m28s y Web 12m02s, mismas identidades y gates; se integró como `8ce25fd`.
  Su confirmación natural aprobó CI en 7m26s y Web en 11m55s, con el mismo inventario.
- Reingreso y borradores: PR57 quedó integrada como `1eae3ef`. El monitor
  autenticado bloquea peticiones, captura antes del desmontaje y consulta la
  sesión al volver a una pestaña visible. Recupera metadatos, expedientes, carga
  principal y nuevas versiones después de autorizar de nuevo, sin reenviar
  escrituras. Su cabeza aprobó CI en 9m26s y Web en 10m47s, con 3381 pruebas Rust,
  447 controladas y 51 reales. El navegador natural de main agotó el tiempo de
  un recorrido de etapas; PR58 trasladó esa familia a VPS3 y acotó las arenas
  del decoder nativo que falló después en una comprobación de arranque.
  PR58 se integró como `14e7eec`, después de aprobar CI en 8m20s y Web en 11m46s:
  3382 Rust, 447 controladas y 51 reales. CI y Web de su confirmación natural
  también aprobaron; los tiempos e inventarios quedan en el informe de verificación.
  La ampliación de participantes manuales, identidades y fichas tipificadas quedó
  integrada por PR60 como `21729ce`. CI, Web y Documents naturales de main
  aprobaron en 9m21s, 12m45s y 1m15s, con 3457 Rust, 482 recorridos controlados
  y 52 reales; se conservaron las identidades previas y cobertura 97/95/93 %.
  Audiencias/resultados y hechos se integraron por PR62 como `83bc9f7`.
  Su confirmación natural aprobó CI en 8m11s, Web en 14m12s y Documents en
  9m14s, incluyendo espera de cola: 3524 Rust, 511 controladas y 52 reales,
  con cobertura 97/95/93 % e identidades previas conservadas. La recuperación
  de miembros y calendarios se integró por PR63 como `710c41b`; su confirmación
  natural falló en el helper de preparación de un plazo contextual. PR64 corrigió
  la espera de esa respuesta y se integró como `9ad4539`. Su confirmación natural
  aprobó CI en 655 s y Web en 883 s, con 3524 pruebas Rust, 524 recorridos
  controlados y 52 reales. Recursos y asociaciones quedaron integrados por PR66;
  los plazos contextuales, por PR69, y los ordinarios, por PR71, con el cierre
  natural corregido por PR72. La recuperación de informes y preferencias quedó
  integrada por PR74 y PR75, con confirmación natural aprobada. PR77 integró
  confirmaciones administrativas y de sellado después de aprobar sus tres gates;
  su confirmación natural también está aprobada. La lectura de avisos, con cuatro
  recorridos locales de vencimiento aceptados, se integró por PR81 y tiene
  confirmación natural de CI, Web y Documents aprobada.
  Un recorrido con servicios desechables aceptó dos vencimientos y tres MFA,
  conservando campos de expediente y archivo principal. El inventario explícito
  de editores todavía pendientes está en [recuperación de editores](session-editor-recovery.md).
  La duración operativa sigue sin aprobarse; estos resultados no habilitan
  inactividad en VPS3 ni demuestran recuperación universal de formularios.
- Alta de integrantes, integrada por PR85: recupera correo crudo y rol de la misma cuenta Owner sin contraseña ni material MFA. Conserva
  el intento incierto al navegar y exige consulta de autoridad antes de mostrarlo;
  ni presencia ni ausencia en el directorio permiten reenviarlo. El registro
  aprobó 20/20 casos Node en 298.159 ms y el navegador 9/9 en 22.5 s, incluidos
  ocho nuevos y la regresión móvil de alta MFA/auditoría. Esos focales usan HTTP
  controlado; CI, Web y Documents de la PR aprobaron. No hay aceptación nueva
  del alta con servicios reales ni despliegue. Véase
  [recuperación de altas](member-enrollment-recovery.md). Invitaciones y
  recuperación del enrolamiento conservan su condición pendiente.
- Recuperación de contraseña: la frontera interna está integrada por PR59 como
  `f29ea91`, con emisión de capacidad de un solo uso y cambio de contraseña,
  generación y auditoría en una transacción. Su cabeza aprobó CI en 9m21s, Web
  en 12m26s y Documents en 1m15s: 3457 pruebas Rust, 447 de navegador controlado
  y 51 reales, con cobertura 97/95/93 % e identidades anteriores conservadas.
  La confirmación natural de main aprobó CI en 10m26s, Web en 12m02s y Documents
  en 1m15s, con el mismo inventario y gates. La invalidación
  administrativa de capacidades restauradas tiene doce focales aprobadas.
  Transporte de correo, rutas, formulario y consumidor del servidor tienen
  verificación local separada y se integraron por PR61 como `4ea2fba`.
  CI, Web y Documents naturales de main aprobaron en 11m26s, 12m36s y 1m16s,
  con 3524 Rust, 493 controladas, 52 reales y cobertura 97/95/93 %.
  Permanecen deshabilitados en VPS3.
  La aceptación HTTP integrada aprobó con PostgreSQL, Redis, entropía OS,
  Argon2id y MFA reales; sólo la entrega fue capturada. Faltan la
  configuración operativa y coordinación del controlador de
  restauración antes de habilitar la recuperación pública.
  Véanse [contrato interno](password-reset-internal.md) y
  [transporte público](password-reset-public.md).
- Vínculo de certificado Owner: dominio, verificación RSA, aplicación,
  persistencia PostgreSQL auditada, rutas HTTP y composición del servidor tienen
  aceptación local por capa. Conservan declaración canónica, firma separada,
  recibos exactos y retirada terminal con autorización vigente. La campaña
  completa `scripts/api-demo.sh` aprobó en 334.419 s con HTTP, PostgreSQL,
  Redis, MFA y RSA reales; tras restaurar SQL y renovar MFA conservó el recibo
  terminal, la prueba pública y la auditoría exactos. El dominio quedó integrado
  por PR70 como `fb8b01e`, con confirmación natural aprobada; el verificador,
  por PR73, con confirmación natural aprobada. Aplicación, persistencia/HTTP
  compuesto e interfaz quedaron integrados por PR78, PR82 y PR83 respectivamente,
  con sus confirmaciones naturales aprobadas; no se instalaron en VPS3. La interfaz
  Qadra posterior aprobó seis recorridos con HTTP controlado en 16.8 s; conserva
  intención y recibos públicos, sin recibir claves privadas. La aceptación
  del navegador con servicios reales aprobó después 1/1 en 12.1 s, con
  PostgreSQL, Valkey compatible con Redis, MFA y OpenSSL reales. El comando
  completo duró 252.134 s; no repitió la restauración anterior.
  La consulta posterior `/current` descubre el recibo propio sin retirar o
  su ausencia, con aceptación focal separada de aplicación, HTTP y PostgreSQL;
  no formó parte de aquella campaña HTTP/restauración, pero sí del navegador
  real posterior. Descubrirlo no acredita vigencia del certificado.
  El vínculo por sí solo no habilita autenticación por certificado ni firma
  documental individual. Véanse
  [ADR-0067](adr/0067-owner-certificate-bindings.md) y
  [el contrato HTTP](http-owner-certificates.md).
- Primer factor Owner, integrado por PR84: aplicación con MFA obligatorio,
  autoridad vigente PostgreSQL, verificador tipificado y capacidades Redis con
  procedencia explícita. Aprobaron 22 casos de aplicación, 31 previos de identidad,
  ocho criptográficos, seis SQL, 11 de procedencia, 23 regresiones de sesiones y
  21 de captura/presupuestos. El router opcional posterior aprobó 19 casos de
  transporte; su prueba concede sólo MFA y comparte el presupuesto de la API.
  La composición ejecutable posterior aprobó siete casos y un recorrido nativo
  RSA/MFA/PostgreSQL/Redis con confianza sucesora y retirada. Permanece opt-in
  y deshabilitada por defecto. El cierre HTTP y de restauración posterior aprobó
  en 365.645 s, conservando vínculos y controles e invalidando las capacidades
  antiguas antes de expirar. La interfaz posterior aprobó 36 pruebas Node, cinco
  recorridos controlados y siete regresiones previas. El acceso con firma externa
  y MFA reales aprobó 1/1 en 14.0 s (208.382 s con preparación),
  preservando recibos y el retiro posterior bajo contraseña independiente.
  La confirmación natural de CI, Web y Documents está aprobada. La activación
  operativa sigue pendiente y permanece deshabilitada en VPS3. Véanse
  [autenticación interna](owner-certificate-authentication.md) y
  [recorrido de acceso](owner-certificate-login-interface.md).
- Informes integrados por PR49: solicitudes propias durables, captura cifrada,
  PDF/CSV, avisos internos, selector paginado y consumidor supervisado están
  integrados en `main`. Se acreditaron trece escenarios distintos de
  navegador controlado; las mediciones de capacidad se detallan más adelante.
  Dos recorridos con servicios reales aprobaron y conservaron PDF/CSV exactos
  tras renovar la sesión. La aceptación HTTP/restauración aprobó en 326.299 s
  con artefactos y avisos exactos, reinicios y revocación de toda la captura.
  Los gates de PR49 y su confirmación natural en `main` aprobaron.
  Los filtros usan creación y estado actual,
  no un censo histórico. Correo, estimación de terminación, otros tipos de
  informe y métricas de desempeño permanecen pendientes del alcance aprobado.
  La duración observada se integró por PR52 en `2621755`, con CI 9m26s, Web
  10m30s y Documents 1m14s; conserva 3340 pruebas Rust, 419 controladas y 51
  reales. La confirmación natural de main aprobó y esta ampliación quedó
  incluida en `v0.1.1`, con identidad y salud comprobadas. Ese tiempo observado no estima la terminación.
- Consulta de actividad integrada por PR50: selección Owner por intervalo UTC,
  actor histórico, operación y recurso, con paginación de instantánea y lectura
  auditada. Aplicación, PostgreSQL, HTTP, cliente y navegador controlado aprobaron.
  La aceptación API/restauración aprobó en 376.459967 s y el navegador real,
  2/2 en 13.7 s; CI y confirmación natural de `main` aprobaron tras integrar PR50.
  Está incluida en `v0.1.0`; falta la aceptación autenticada de esa instalación.
  CU-17/RF-20 conservan
  pendientes identificador estable de cuenta, IP y el criterio de registro
  menor de 500 ms; el anclaje externo sigue separado.
- Pendientes operativos: firma personal y calificación jurídica de urgencia
  siguen separadas del tablero. Ni Inicio ni verificar la cadena completan
  esos flujos o el alcance restante de la bitácora.
- Despliegue: PR43 conserva la propuesta de la compañera y añade la adaptación
  a VPS3. Se integró como `3a67a6a` después de aprobar CI, Web y Documents,
  con confirmación natural de main aprobada: CI 6m42s, Web 11m25s y Documents
  7m43s. La release `v0.1.0` aprobó el workflow de despliegue en 17m28s,
  incluyendo 6m05s de empaquetado y 16s de activación. PostgreSQL, Redis, API y
  frontend están activos en loopback; salud, identidad y PKI se comprobaron y
  se capturó un respaldo posterior a inicializar las claves. Su copia privada
  fuera del host y restauración SQL/PKI aprobaron con cero usuarios y un evento
  de auditoría; ese respaldo inicial no contiene RDB. Los controladores
  `673b9ec`, instalados bajo `deploy.lock`, generaron después la captura
  `20261002T105313Z-886940db` con SQL, RDB, PKI y `COMPLETE` validados. Su copia
  externa de cuatro archivos coincidió por hash y tamaño. Su restauración
  conjunta SQL/RDB/PKI con PostgreSQL 16.15 y Redis 7.4.11 aprobó, incluida
  persistencia AOF, confianza y auditoría; conservó cero usuarios y Redis vacío.
  La captura y restauración poblada tienen ahora aceptación local separada:
  1/1 en 27.424 s con dos usuarios, dos asignaciones, cuatro estados de reset,
  invalidación idempotente, cadena y PKI restauradas y reinicio Redis sin
  resurrección. Se usó un paquete interno desechable y observación systemd
  sustituida, sin cambiar la instalación. La admisión de compatibilidad se integró
  por PR65 como `ffdee8f` el 3 de octubre a las 05:08:42 UTC. Su cabeza aprobó
  CI en 513 s con 3524 pruebas Rust y Documents en 76 s; Web no aplicaba.
  La confirmación natural de main aprobó CI en 513 s y Documents en 75 s,
  con las mismas 3524 pruebas Rust; Web no aplicaba. La captura posterior se
  integró por PR67 como `a208051` y su corrección de observación por PR68 como
  `c95f245`, cuya confirmación natural aprobó CI en 443 s. La parada
  observada y reentrante y la publicación de controladores tienen aceptación local
  separada; la primera incluye cuatro unidades systemd desechables, sin operar
  los servicios instalados. Estas ampliaciones aún requieren integración e
  instalación y no acreditan staging, promoción o reapertura del producto instalado.
  Owner espera el correo elegido por el operador; siguen cero usuarios y la
  aceptación autenticada pendiente.
- Instalador recuperable de controladores: tiene aceptación local; la publicación
  e integración del grupo completo y su instalación en VPS3 siguen pendientes.
  La composición local aprobó 10 casos en 4.159 s y 52 regresiones en 3.021 s; el bootstrap,
  ocho en 1.703 s. El ensayo nativo aprobó 1/1 en 3.461 s con cuatro servicios
  inocuos de `tt-runner`, generaciones A/B, launcher real y reapertura explícita.
  Sus sondas de datos son sintéticas: no acredita PostgreSQL/Redis reales ni
  restauración o instalación operacional. Véase
  [instalación de controladores](deployment-controller-installation.md).
- Mantenimiento de CRL: el controlador manual tiene 96 pruebas aprobadas y
  aceptación aislada PostgreSQL/OpenSSL de tres renovaciones, respuesta perdida
  y recuperación tras fallo de salud. Preserva autoridad, claves, series revocadas
  y auditoría. Los nueve controladores ya están instalados en VPS3 y la renovación
  operativa aprobó 172 comprobaciones en 8.246 s, con respaldo y cadena de auditoría
  verificados. La programación automática sigue pendiente; véase
  [el procedimiento](deployment-crl.md).
- Cierre académico posterior: conciliación del manuscrito con las entregas,
  evaluación formal con participantes reales y conclusiones basadas en resultados.

El tablero operativo implementa una instantánea autorizada y su interfaz Qadra,
con aceptación focal y restauración aprobadas. La
[PR 45](https://github.com/eddndev/TT2026-B136/pull/45) ya está integrada y su
regresión natural en `main`, revisión `0cf1f295`, aprobó CI, Web y Documents;
su contrato está en
[dashboard-api.md](dashboard-api.md).

La entrega integrada de
[creación contextual de plazos](resource-deadlines-api.md): una confirmación
registra el plazo y su vínculo al recurso dentro de una transacción auditada.
La aceptación integrada con restauración y la conciliación de respuestas
inciertas en Qadra ya aprobaron localmente. La
[PR 46](https://github.com/eddndev/TT2026-B136/pull/46) aprobó los tres gates y
se integró por squash como `23970cc516854688da587e878d4304fa52068ecf`. Su
confirmación natural de main aprobó también CI, Web y Documents, conservando
3084 pruebas Rust, 378 de navegador controlado y 45 con servicios reales. Este registro explícito no completa la activación jurídica
automática ni la creación de audiencias de
recursos. Ninguna de estas entregas cambia los objetivos aprobados ni declara
completos los otros pendientes.

La consulta de actividad se integró por PR50 como `a8dc3dd`, después de aprobar
CI, Web y Documents sobre `b8e4a53`. Su confirmación natural aprobó en 8m44s,
10m37s y 1m15s, respectivamente: 3340 pruebas Rust, dos ignoradas, 418 de
navegador controlado y 51 reales, con las identidades anteriores conservadas y
cobertura 97/95/93 %. Esto cierra esa entrega de consulta; no completa la captura
uniforme de actor/IP, el anclaje externo ni los demás requisitos de bitácora.

## Avance del cronograma y aceptación de informes

La semana 9 continúa en cierre funcional: los incrementos integrados de tablero,
plazos contextuales, admisión documental y consulta inversa de recursos se
incluyen ahora los informes integrados y confirmados en `main`; la consulta de actividad integrada por PR50 también confirmó su
regresión en `main`. La primera activación privada del despliegue está comprobada; falta su
aceptación autenticada. La semana 10 mantiene
la evaluación con participantes reales, consolidación de manuales, conciliación
final del manuscrito y conclusiones derivadas de resultados. Preparar un manual
no completa el estudio de usabilidad; no se asigna un porcentaje nuevo por
contar commits o pruebas.

En informes se acreditan trece escenarios distintos de navegador con HTTP
controlado y dos recorridos con servicios reales. Owner y Litigante aprobaron en
10.6 s y 7.6 s; el ejecutor duró 23.4 s y la campaña completa 207.862628704 s.
Las cuatro capturas de escritorio/móvil (1440/390 píxeles) fueron inspeccionadas.
Las descargas de PDF y CSV conservaron sus bytes y digests tras nueva autenticación.

La reutilización del plan tipográfico aprobó dos pruebas de igualdad exacta de
glifos y posiciones. El PDF representativo de 60 expedientes y 25 páginas pasó de
2.7404 s a 1.604057 s, con bytes idénticos y SHA-256
`32659e0008fefdf7565ba66d60708cc3e117871e11d4affb23bfb26c808a4674`.
La captura de 1000 expedientes, 10000 asignaciones y 1000 entradas de carga
conserva su CSV aprobado en 0.382 s; PDF volvió a devolver `CapacityExceeded`
en 15.197 s. No se atribuye el recurso agotado ni se afirma generación del PDF
máximo. Las páginas 1/2/3/13/25 del PDF representativo tienen inspección visual
aprobada, sin cambio de los presupuestos existentes.

La aceptación HTTP/restauración terminó con salida cero en 326.298950161 s,
con 3136 archivos fuente sin cambios durante la ejecución. Creó tres trabajos y
comprobó idempotencia, PDF/CSV exactos, identidades, avisos y acuse explícito;
Client y Paralegal fueron rechazados. En los trabajos de avisos se observaron
`queued -> processing.rendering -> ready`; en el posteriormente revocado,
`queued -> ready`. No se declara observada la fase de captura.

Los reinicios TERM e INT aprobaron en cinco segundos cada uno y conservaron el
histórico exacto; antes TERM había tardado 59 s e INT había excedido 60 s. Tras
restaurar PostgreSQL y renovar MFA permanecieron los bytes de ambos formatos,
sus identidades, los avisos leídos/no leídos y el rechazo de la captura revocada
completa. El arranque optimizado tiene evidencia focal separada: ocho pruebas
PostgreSQL, dos comprobaciones de compilación negativa y 43 de composición.
La cabeza `6e1ae2b` de PR49 aprobó CI en 9m13s, Web en 12m32s y Documents en
10m16s: 3299 pruebas Rust y dos ignoradas, 409 de navegador controlado y 49
reales; cobertura de dominio/aplicación/infraestructura de 97/95/93 %. Se
integró por squash como `f3bfed8`; su confirmación natural de `main` aprobó
CI en 8m36s, Web en 11m46s y Documents en 1m13s con el mismo inventario. Esta evidencia no corresponde al incremento de consulta de actividad.
CU-16 y RF-19 se conservan literalmente: avisos internos no sustituyen correo ni
una estimación de terminación; carga de asignaciones no equivale a desempeño
jurídico.

## Alcance y evidencia

El cierre comprende gestión documental, gestión procesal penal, interfaz web,
conciliación de alcance y validación del producto. Los objetivos aprobados se
conservan en [la introducción](../latex/chapters/01-introduccion.tex); el catálogo
funcional está en [análisis y diseño](../latex/chapters/03-analisis-diseno.tex).
El [informe de verificación](verification-report.md) distingue resultados
reproducidos de mediciones históricas. Este documento distingue capacidades
implementadas y trabajo pendiente; la matriz no sustituye su evidencia.

La aplicación `web/` integra la identidad Qadra con expedientes reales, rutas
autorizadas y consultas persistentes. Ofrece carga inicial, historial y selección
de versiones, sellado, verificación y descarga exactos. La clasificación actual
tiene tipo, clasificación, etiquetas y revisiones auditadas independientes del
contenido, con carga inicial atómica y filtros exactos. Las pruebas con HTTP
simulado y los recorridos contra servicios reales tienen evidencias separadas.
`frontend/` conserva el ejemplo anterior; el producto continúa en `web/`.

El [directorio de miembros](members-api.md) y la administración de acceso están
integrados en `main` en aplicación, PostgreSQL, HTTP y Qadra. Owner consulta
cuentas sin secretos, filtra por correo/rol/estado y selecciona asignados o
disponibles por expediente. El cambio conjunto de rol y estado exige revisión
esperada, conserva al último Owner activo y revoca sesiones y desafíos anteriores
mediante generación durable. Reactivar conserva las asignaciones y exige nuevo
inicio de sesión. Aplicación/HTTP, Node y navegador con HTTP controlado tienen
evidencia focal aprobada. PostgreSQL/Redis también aprobó sus 12 casos, incluida
la restauración. Tres recorridos con navegador real aprobaron en 418.426 s de
comando y 40.1 s de Playwright, con 2553 fuentes estables: escritorio a 1440 y
móvil a 390 píxeles, expediente cerrado, revocación del token anterior tras
reactivación y restricciones por rol, incluido Client. Las cuatro capturas fueron inspeccionadas: legibles, sin desbordes ni
solapamientos y consistentes con Qadra. Esta inspección no es una prueba de
usabilidad con personas. La aceptación API completa con restauración aprobó en
453.174 s con 2553 fuentes estables: preservó cuentas y asignaciones, rechazó
tokens anteriores y verificó nuevo acceso MFA y evidencia documental exacta.
Ese incremento y su cierre global se integraron en `main`. Las cifras anteriores describen su aceptación original, no una nueva ejecución.
Invitaciones y autenticación por certificado siguen fuera de esa implementación.
La recuperación de contraseña se integró posteriormente por PR61; conserva
sus condiciones de activación operativa y entrega externa.

La entrega integrada de [contenido original e incidentes](document-content-api.md)
añade descarga de versiones pendientes o selladas, comprobación completa antes
de entregar bytes y aviso interno persistente a Owner. Su aceptación y cierre
global se conservan en el informe como evidencia de esa entrega. La política
Client por recurso y la identidad individual para firma permanecen pendientes.

La [admisión general](document-upload-admission-api.md) está integrada
para nuevas cargas y versiones: PDF, DOCX, TXT, JPEG, PNG, MP3, WAV
y MP4, hasta 16 MiB, con inspección del contenido y decodificación multimedia
completa en procesos acotados. Nombre, extensión, MIME y clasificación no
seleccionan el formato. Los bytes originales se conservan y la identidad se
revalida antes del commit auditado; Qadra mantiene archivo y borrador ante un
rechazo, sin reenvío automático. Aplicación, HTTP, cliente, navegador controlado,
adaptador nativo y supervisor tienen evidencia focal aprobada. La aceptación
API completa terminó con salida cero y restauró las ocho familias con bytes
exactos. El navegador real aprobó el escenario nuevo y tres regresiones de
contenido (4/4, 27.2 s); se inspeccionaron las capturas a 1440 y 390 píxeles sin
desbordamiento horizontal. Clippy y la dependencia en los tres VPS aprobaron;
el PDF de 343 páginas se compiló e inspeccionó. El incremento quedó integrado
mediante [PR47](https://github.com/eddndev/TT2026-B136/pull/47), commit `ac34b34`,
y su confirmación natural de `main` está comprobada. Las cifras anteriores
conservan el corte de aceptación original y no son ejecuciones nuevas. VPS1 utiliza una compilación nativa para su ABI.
La política de soportes
procesales PDF/DOCX permanece independiente: los nuevos formatos no amplían
los soportes permitidos de un acto ni se aplican retrospectivamente. Véase
[ADR-0058](adr/0058-bounded-general-document-admission.md).

El directorio por expediente agrega fichas independientes de las cuentas, valores
manuales o tipificados, archivo/reactivación e historial inmutable con autoría. El rol de acceso
y la asignación autorizan cada operación; el texto del rol procesal no concede
permisos. Su base y límites se describen en
[ADR-0021](adr/0021-audited-case-participants.md). Este avance no cierra los
criterios de identidad jurídica ni la gestión procesal completa.

Las fichas tipificadas vinculan una revisión exacta de identidad representada y
uno de once perfiles. La revisión de posibles duplicados usa señales declaradas,
soportes y huellas de certificados, con decisiones explícitas y control de
concurrencia. Qadra permite completar fichas manuales, consultar la identidad
actual por separado y recuperar evidencia pública de declaraciones firmadas
externamente. La CA interna y una CRL publicada delimitan esa demostración:
no equivalen a FIREL ni acreditan civilmente una identidad o una profesión.
Los contratos están en [ADR-0025](adr/0025-case-subjects-and-typed-participants.md),
[ADR-0026](adr/0026-internal-participant-declarations.md) y
[la API tipificada](typed-participants-api.md).

El registro penal completo añade NUC, carpeta judicial, autoridades, delitos e
información general, con revisión administrativa y registro inicial de
Investigación. La edición, el cierre administrativo y la reapertura conservan
la historia y la etapa inicial. Completar una ficha anterior no inventa etapas;
la API básica mantiene su proyección para Client. La administración y el cierre
se describen en [ADR-0022](adr/0022-audited-penal-case-administration.md).

La adopción explícita y los avances Investigación a Intermedia e Intermedia a
Juicio tienen dominio, aplicación, persistencia auditada, HTTP y flujo Qadra.
Cada soporte fija documento, versión y digest; se comprueban integridad y formato
PDF/DOCX en un proceso acotado. Se distinguen registro inicial histórico, etapa
actual, actos declarados y captura del sistema. Perfil completo, estado activo,
rol y asignación se revalidan al confirmar; cierre y revocación no se eluden con
una preparación anterior. Lectura e historia siguen disponibles en expedientes
cerrados según permisos. El diseño está en [ADR-0023](adr/0023-audited-case-stage-transitions.md)
y [ADR-0024](adr/0024-isolated-document-format-admission.md).

La programación de audiencias incorpora cuatro tipos, referencias históricas de
participantes, soporte exacto de antecedente para individualización, reemplazos
y cancelación organizativa. Qadra conecta el directorio del expediente con una
agenda transversal de audiencias autorizada. Las comprobaciones por capa, la
campaña global, HTTP, restauración y navegador real se distinguen en el
[informe de verificación](verification-report.md). Su contrato y límites están en
[la API de audiencias](hearings-api.md) y [ADR-0028](adr/0028-audited-hearing-scheduling.md).

Las sesiones y resultados declarados añaden raíces independientes con ancla de
programación y continuidad exactas, comparecencias históricas, acuerdos ordenados,
procedencia y precisión temporal conservadas. La rectificación y el retiro
mantienen historia y recibos propios; Qadra permite prepararlos y consultarlos.
La verificación local comprende pruebas por capa, campaña global, API con
restauración e interfaz simulada y real; sus métricas se conservan en el informe
de verificación. No activan plazos ni alertas y no acreditan actos judiciales. Véanse [el contrato](hearing-results-api.md) y
[ADR-0029](adr/0029-declared-hearing-sessions.md).

El backend, la API y Qadra para el catálogo de calendarios jurisdiccionales están
verificados localmente, incluida la restauración y los recorridos de navegador
con servicios reales. Sus
revisiones conservan ámbito global, fechas civiles, cobertura,
patrón semanal, excepciones y referencias públicas declaradas. Owner gestiona;
el personal consulta sin membresía de expediente y Client queda denegado.
La clasificación distingue exclusión, falta de resolución y falta de cobertura,
sin fabricar disponibilidad o efectos jurídicos. La cobertura y las campañas
web están aprobadas localmente y el código está integrado en `main`; la
actualización integral del manuscrito sigue pendiente. Véanse
[el contrato](judicial-calendars-api.md) y [ADR-0030](adr/0030-versioned-jurisdictional-calendars.md).

El [conteo civil diario](deadline-day-counting.md) ya calcula una fecha candidata
sobre valores exactos de calendario y conserva cada dia con su clasificacion,
fuente y acumulado. Se detiene ante datos sin resolver, falta de cobertura o
agotamiento del rango de fechas. Sus quince pruebas estan incluidas en la suite
completa reproducida. La primera fecha incluida y la cantidad todavia son
entradas matemáticas. El registro persistente ya las vincula a una revisión
exacta de perfil, fuentes y calendario, y conserva la evaluación y su atención.
La calificación del supuesto no se deduce de esa aritmética; el cierre del
corpus jurídico y la activación automática siguen pendientes.
La reevaluación compuesta en servidor está integrada por PR 34. Las alertas
se integraron por PR 36 como `8261c51`.

La [aritmetica temporal](deadline-arithmetic.md) implementa reglas matematicas
explicitas para dias naturales o computables, meses civiles y horas transcurridas.
Conserva insumos y trazas, aplica el ajuste final solo si se solicita y bloquea
ante precision insuficiente, homologo inexistente o calendario incompleto.
No selecciona un supuesto juridico ni crea un plazo persistente. Los
[perfiles normativos](deadline-rule-research.md) todavia requieren cerrar
aplicabilidad, inicio, duracion ordenada y corte, particularmente para meses.
La activación automática sigue pendiente. La reevaluación compuesta en servidor
está integrada por PR 34; las alertas se integraron por PR 36 y conservan su
propia evidencia de aceptación en el informe de verificación.

La [extracción temporal exacta](deadline-triggers.md) comprueba expediente,
identidad, revisión, padre y acuerdo del material recibido antes de seleccionar
un campo. Distingue ausencia y desconocimiento, conserva precisión y procedencia,
y requiere una calificación expresa cuando se necesita un tiempo de otra
finalidad. El coordinador entrega el tiempo exacto a la aritmética. Es un componente
de dominio, integrado ahora mediante la [resolución autorizada](deadline-inputs.md)
de fuentes y calendarios en PostgreSQL. El servicio verifica recibos, conserva
selección histórica y cabeza observada, y reautentica antes de devolver el cálculo.
El [catálogo de perfiles](deadline-profiles-api.md) agrega configuraciones
versionadas globales o por expediente, condiciones, corpus reproducible, retiro
terminal y recibos auditados. El evaluador puro combina perfil, fuentes comprobadas
y declaraciones de aplicabilidad; conserva candidatas y bloqueos y exige un
corte explícito para obtener un instante civil. La API pasó las pruebas con puertos
controlados y el recorrido con persistencia real y restauración.

El [registro de plazos](deadline-records.md) añade evaluación persistente por
expediente, responsable, atención declarada, corrección y retiro, con historia
inmutable y auditoría transaccional. PostgreSQL conserva entradas, resultados,
fuentes y cabezas exactos; la consulta histórica no recalcula. La
[API de plazos](deadlines-api.md) está implementada y sus pruebas focales están
aprobadas. La aceptación HTTP integrada con restauración y la campaña global
del backend aprobaron. Ese bloque fue integrado en `main` mediante
[PR 32](https://github.com/eddndev/TT2026-B136/pull/32), commit `ae205d2`.

La interfaz Qadra de plazos incorpora:
registro, corrección, declaración de atención, retiro, consultas e historia por
revisión exacta. Los selectores conservan perfil, fuente, padre, acuerdo y
calendario exactos; el formulario no rellena cantidades ni condiciones
jurídicas. El selector paginado de responsables limita candidatos a cuentas
activas con acceso al expediente y confirma auditoría; no sustituye el directorio
general de miembros ni asigna acceso. La preparación muestra bloqueos y traza,
exige confirmación expresa y concilia respuestas inciertas por revisión exacta
sin reenviar. Owner y Litigator gestionan, Paralegal consulta, Client queda
excluido y el cierre conserva las lecturas autorizadas. La
[guía Qadra](../web/README.md#plazos-del-expediente) describe las operaciones.

Las campañas locales completas del selector y la interfaz están aprobadas:
Rust, contratos y formularios, regresión con HTTP controlado, API/restauración
y navegador con servicios reales. Sus resultados y la comprobación remota se
registran por separado del backend anterior en el informe de verificación.
La [PR 33](https://github.com/eddndev/TT2026-B136/pull/33) integró esta interfaz
y el selector en `main`, commit `214202a`, el 18 de septiembre de 2026 a las
15:09 de Ciudad de México, con 19 comprobaciones aprobadas y el paso de release
omitido conforme al evento.
La reevaluación compuesta y Qadra V2 tienen recorridos con backend real aprobados.
La aceptación API final y el cierre global aprobaron; PR 34 integró esta entrega
y PR 35 integró la agenda combinada. La activación automática sigue pendiente;
las alertas se integraron por PR 36.
Los cambios de fuentes ya tienen despacho y consumo mediante puertos internos.
Tampoco los ejemplos
sintéticos cierran los perfiles jurídicos con fundamento y casos de aceptación
aplicables al alcance aprobado.

Sobre la base V1 integrada, la ampliación V2 implementa seguimiento durable,
un servicio humano único y su contrato HTTP. Servicio, HTTP e interfaz Qadra V2
están implementados; el dispatcher/worker está compuesto en servidor y tiene
verificación API real con reinicios y restauración. Qadra V2 también tiene una
campaña de navegador con backend real aprobada. La aceptación API final aprobó;
el cierre global aprobó y esta ampliación se integró mediante la
[PR 34](https://github.com/eddndev/TT2026-B136/pull/34). Véanse
[los recibos de seguimiento](deadline-tracking-receipts.md),
[el contrato HTTP](deadline-tracking-api.md) y
[ADR-0037](adr/0037-durable-deadline-reevaluation.md).

- Evidencia V1/V2, autoría humana/técnica explícita y continuidad con ambas
  huellas del predecesor, conservando una sola evaluación histórica.
- Observaciones verificadas de perfil, fuente, calendario y padre independiente
  de notificación. El legado usa sólo la evidencia capturada, sin inventar una
  cabeza observada ni declarar aceptación.
- Alta y corrección humanas con políticas explícitas; atención y retiro conservan
  seguimiento y cálculo. El autor captura la cuenta y correo autenticados;
  la reautenticación compara la identidad completa y el commit revalida recibos,
  observaciones y administración bajo autorización y auditoría atómicas.
- Despacho durable y consumidor que autentica la causa exacta, vuelve a comprobar
  la base y las cabezas, y confirma revisión y resultado o evidencia sin cambios.
  Conserva responsable, atención, políticas, calificaciones y motivos pendientes;
  sólo recalcula al avanzar un calendario seguido bajo las condiciones de aceptación.
- Detalle actual y listado que comparan cabezas verificadas con las observaciones
  capturadas bajo la misma lectura autorizada y auditada. Separan fecha calculada
  de vencimiento operativo; sólo un plazo activo, aceptado, con instante y vigencia
  comprobada lo expone. Cola pendiente, `Completed` y resultados sin cambios no
  certifican frescura. Los cambios ordinarios bajo `Fixed` conservan vigencia
  con cabeza activa; el retiro requiere revisión y la evidencia regresiva se rechaza.
- Historia y HTTP representan de forma explícita recibos V1, seguimiento V2,
  revisión pendiente, causa y autoría técnica. El puerto humano no permite
  solicitar revisiones técnicas. Lectura e inventario reconstruyen fuentes y
  administración exactas y rechazan evidencia falsa aun con hashes coherentes.

La preparación técnica publicada en la PR 34, commit `8c838cb`, tiene una
campaña propia de 2442 pruebas Rust aprobadas y controles globales aprobados.
La evidencia de esa cabeza y las campañas anteriores se conserva en el
informe de verificación; no acredita los cambios posteriores de almacenamiento.
La restauración HTTP histórica comprueba V1.

Las campañas completas de almacenamiento y del consumidor tienen resultados
propios en el [informe de verificación](verification-report.md). La regresión
HTTP con servicios reales y restauración registrada en esas entregas ejercita
V1. La ampliación humana/HTTP V2 aprobó 50 pruebas focales de aplicación,
160 de web Rust, 17 de PostgreSQL y una de lectura concurrente, además
de Clippy en los tres crates y todos sus targets. Qadra V2 aprobó 88 pruebas
Node y 36 recorridos de navegador con HTTP controlado, incluidos ocho nuevos
de escritorio y móvil. La inspección visual cubrió las nuevas vistas diarias
y horarias a 1440 y 390 píxeles, sin desbordamiento. La campaña posterior de
navegador con backend real aprobó 25 recorridos, incluidos dos Follow de escritorio
1440 y móvil 390; se inspeccionaron seis capturas. Esa campaña comprueba cambios
de calendario, revisión pendiente por fuente, corrección humana e historia.
Estos grupos tienen alcance propio y no constituyen una nueva regresión global.

La reconexión del despachador aprobó cuatro pruebas; junto con cinco de
atomicidad y dos de presupuestos, su regresión focal suma once aprobadas.
El ejecutable aprobó 31 pruebas unitarias y cuatro de ayuda CLI: 35 en total.
Las 13 del bucle, cuatro de parada y nueve del supervisor suman 26 incluidas
en las 31 unitarias. La composición en `serve` valida ambos adaptadores antes
de escuchar y mantiene un bucle bloqueante serial y parada coordinada.
La aceptación API final terminó con código cero tras corregir la comprobación
del token de recuperación: comprobó reevaluación, cierre TERM/INT, reinicios y
conservación exacta de R1-R5 tras restauración. La demostración CLI también
aprobó. El [informe de verificación](verification-report.md) conserva tiempos,
fuentes y alcance de estas ejecuciones y del cierre global aprobado de PR 34.

La [PR 34](https://github.com/eddndev/TT2026-B136/pull/34) integró el consumidor
y la ampliación humana/HTTP/Qadra V2 mediante squash `e2e6758`. El
[despachador](deadline-dispatch.md) expande eventos y legado en trabajos
persistentes; el [consumidor](deadline-worker.md) confirma revisión, resultado
y auditoría, o resultados sin cambios e intentos con espera durable. Los
controles globales y los recorridos reales de esta entrega se conservan en el
informe de verificación. La agenda posterior se integró por separado mediante
la [PR 35](https://github.com/eddndev/TT2026-B136/pull/35), squash `c16b820`.
Las [alertas durables](alerts-api.md), sus preferencias, bandeja Qadra y
consumidores compuestos en `serve` se integraron en `main` como `8261c51` por la
[PR 36](https://github.com/eddndev/TT2026-B136/pull/36), tras su aceptación API
con restauración, navegador real y cierre de CI. La evidencia se conserva por
entrega en el informe de verificación.

La conciliación excepcional de dependencias retiradas requiere un contrato
explícito. La ampliación de reevaluación no cierra activación ni alertas y sus
recorridos aprobados no acreditan la aceptación de la agenda posterior.

La [agenda combinada](agenda-api.md) reúne audiencias y vencimientos operativos
en una consulta autorizada y auditada. El servidor comprueba las dependencias
actuales de cada plazo antes de incluirlo; su fecha calculada histórica no basta.
Qadra ofrece día, semana, mes y rango personalizado con desfase explícito,
filtros por familia y estado de audiencia, y páginas acumulativas. Una página
vacía puede conservar continuación y nunca acredita agotamiento por sí sola.
Las actividades se distinguen por familia e identificador y conservan la revisión
mayor recibida. Abrirlas revalida el expediente y consulta la revisión exacta.
La decisión y los límites se describen en
[ADR-0038](adr/0038-authorized-combined-agenda.md). La agenda está integrada en `main` con aceptación real de escritorio, móvil
y restauración. Sus resultados no sustituyen la aceptación propia de las
alertas y asociaciones ya integradas ni los objetivos pendientes de activación,
audiencias de recursos, identidad y operación del despacho.

La [precision temporal declarada](procedural-time.md) conserva datos desconocidos,
fecha, minuto y segundo, con desfase opcional. Esta implementacion de dominio no
registra hechos por si misma. La separacion de resoluciones y practicas de
notificacion queda adoptada en [ADR-0031](adr/0031-declared-procedural-facts.md),
con contrato y persistencia descritos abajo; la API HTTP esta implementada y
su interfaz Qadra esta implementada y verificada localmente en navegador.
El [modelo puro de hechos](procedural-facts.md) ya conserva las dos familias,
referencias historicas seleccionadas, desconocimiento, funciones personales y
soportes directos. Sus [canones propios](procedural-facts-canonical.md) distinguen
precision y desfase, y conservan cada localizador aunque un documento se comparta.
La [base de aplicacion](procedural-facts-application.md) incorpora comandos de
alta/correccion/retiro, seleccion exacta, comprobaciones puras y contratos de
puertos. El servicio de aplicacion ya coordina autenticacion, verificacion de
fuentes exactas, admision del lote directo, reautenticacion y comprobacion de
recibos de preparacion y respuesta. Sus [canones de fuentes y operacion](procedural-facts-receipts.md)
conservan las vistas historicas y se contrastan con vectores independientes.
El [adaptador PostgreSQL](procedural-facts-persistence.md) implementa ahora
persistencia, autorizacion efectiva por expediente e historia auditada atomica.
La verificacion focal local cubre ambas familias, fuentes exactas, permisos,
concurrencia y restauracion. El cierre anterior del backend incluyo una campana
global local y el guion HTTP de las capacidades ya expuestas; sus resultados
constan en el informe de verificacion.
La [API HTTP de hechos](procedural-facts-api.md) y su composicion estan
implementadas. Pasaron 25 pruebas focales unitarias de entrada/proyeccion y
26 pruebas de rutas HTTP con puertos controlados. La suite global y el recorrido
HTTP con servicios reales y restauracion tambien estan aprobados localmente.
Qadra ya incorpora captura, fuentes historicas, consulta y conciliacion explicita;
su campana de navegador esta aprobada localmente. Sus revisiones exactas pueden
seleccionarse desde el formulario de plazos. La activación automática sigue
pendiente; los avisos están integrados por PR 36 y conservan su aceptación
separada de las campañas de hechos y plazos.

## Entregas y condiciones de cierre

| Entrega | Alcance verificable | Dependencias y evidencia requerida |
| --- | --- | --- |
| Consultas documentales e integración Qadra | Listado paginado, detalle, búsqueda literal de nombre y filtro de sellado; selección y creación de expedientes; carga, sello, verificación y evidencia en el expediente seleccionado. | Autorización vigente antes de exponer metadatos; cuatro roles, expedientes ajenos, revocación, filtros antes de paginación, eventos de consulta y errores. Pruebas HTTP, PostgreSQL y navegador. |
| Versiones documentales | Identidad estable, cifrado vinculado a UUID/versión, evidencia histórica inmutable y selección explícita de snapshot. | ADR-0019, append optimista, migración V1/V7, aislamiento, conflictos, restauración y evidencia byte por byte; resultados en el informe de verificación. |
| Clasificación documental | Tipo, clasificación y etiquetas organizativas con revisiones auditadas; búsqueda autorizada por metadatos. | ADR-0020, carga atómica, canon Unicode, revisiones esperadas, concurrencia, permisos y restauración sin alterar evidencia; resultados en el informe de verificación. |
| Directorio de participantes | Fichas por expediente, revisión esperada, historial, filtros, archivo y reactivación con valores vigentes. | ADR-0021, independencia de cuentas, cuatro roles, asociación ajena, concurrencia, auditoría y restauración. No acredita identidad ni firma judicial. |
| Identidad y firma personal | Resolver autenticación con certificado de socios, identidad del firmante y custodia de claves; altas, bajas, invitaciones y recuperación segura de credenciales. | La clave de firma configurada para el servidor no demuestra firma individual por usuario. Separar recuperación MFA de recuperación de contraseña. Definir contratos y probar revocación con sesiones existentes y fallos de entrega. |
| Administración del expediente penal | Alta penal completa, edición, historia administrativa, filtros, cierre y reapertura; inicial Investigación para nuevas altas completas. | ADR-0022, unicidad de identificadores actuales, R0/R1 pendientes explícitos, cuatro roles, CAS, cierre concurrente, auditoría y restauración completa. |
| Adopción y transiciones de etapa | Adopción para perfiles completos sin etapa; dos avances ordinarios, fechas declaradas y soportes exactos con admisión PDF/DOCX; historia y conflicto explícito en Qadra. | ADR-0023/0024, secuencia y origen de R1, autorización antes y después de preparar, cierre/revocación concurrentes, auditoría, restauración y navegador real. No incluye recursos ni decisiones jurídicas automáticas. |
| Participantes tipificados | Identidades representadas, once perfiles, soportes exactos, revisión de candidatos y declaraciones internas con firma externa. | ADR-0025/0026, unión histórica manual/tipificada, CAS de identidades y fichas, confianza publicada, permisos, cierre, auditoría y restauración. La demostración interna no acredita identidad civil, profesión ni FIREL. |
| Recursos procesales | Resoluciones y soportes exactos, actos e historia propios, audiencias, términos calculados y alertas asociados. | Registro declarado implementado con aceptación real; asociaciones a audiencias/plazos existentes implementadas con evidencia focal y aceptación API/restauración y navegador real aprobados; incremento integrado en main. La creación contextual explícita de plazos aprobó aceptación local y CI y quedó integrada mediante PR46; su confirmación natural de main aprobó CI, Web y Documents. La consulta inversa desde actividad o alerta y el regreso a sus capturas tienen aceptación API/restauración y navegador real aprobados. PR48 está integrada en `5020707`; CI, Web y Documents de su confirmación natural aprobaron. Faltan audiencias contextuales, corpus calificado y activación automática durable, según [el alcance](procedural-resources-scope.md); consultar vínculos actuales no demuestra causalidad del aviso. No es una cuarta transición ni se satisface con documentos o fechas manuales. |
| Programación de audiencias | Cuatro tipos, reemplazo/cancelación con recibos propios, contexto y participantes exactos, historia y agenda autorizada. | ADR-0028; persistencia, autorización, auditoría y Qadra implementados. Evidencias de concurrencia, soporte histórico, resultados inciertos, restauración y navegador en el informe de verificación. No registra celebración, asistentes reales ni acuerdos. |
| Sesiones y resultados declarados | Raíces propias, ancla y continuidad exactas, comparecencias, acuerdos, procedencia, rectificación, retiro e historia; Qadra y persistencia auditada. | ADR-0029; implementado, verificado e integrado en `main`. Fuentes históricas admitidas, soporte readmitido al rectificar, recibos y recuperación; no acredita actos ni efectos jurídicos. |
| Catálogo de calendarios jurisdiccionales | Ámbito inmutable, revisiones, cobertura, reglas semanales, excepciones y referencias públicas; consulta civil exacta y retiro con recibo. | ADR-0030; backend, API, Qadra, cobertura y restauración verificados localmente. Código integrado en `main`; actualización integral del manuscrito pendiente; conservar evidencia de autorización global, canon independiente, concurrencia e inventario. Las referencias no preservan contenido remoto ni acreditan aplicabilidad. |
| Hechos declarados de resolución y notificación | Dos familias por expediente, padre fijo, tiempos y personas declarados, fuentes exactas, corrección, retiro terminal, recibos e historia. | ADR-0031; dominio, aplicación, backend y API implementados. Backend, pruebas focales, suite global y comprobacion HTTP con servicios reales y restauracion aprobados localmente; Qadra implementada, con verificación de navegador aprobada localmente. No acredita efectos jurídicos ni habilita cálculos o recursos. |
| Plazos y calendario | Plazos vinculados, calendario configurable, vencimientos y alertas persistentes. | Aritmética, perfiles, evaluación persistente, responsable, atención e historia están integrados; Qadra V1 y el selector de responsables se integraron por PR 33. Sus campañas globales y API/restauración conservan su alcance histórico. La ampliación V2 implementa observaciones verificadas, continuidad, preparación humana y técnica, persistencia y reconstrucción exacta. Servicio humano y HTTP V2, detalle actual y listado con vigencia están implementados con evidencia focal local. Qadra V2 aprobó 88 pruebas Node, 36 recorridos con HTTP controlado y 25 con backend real, incluidos dos Follow de escritorio y móvil, con seis capturas inspeccionadas. Despacho y consumo durables conservan resultados, intentos e historia. La composición aprobó 31 unitarias y cuatro de ayuda CLI; la campaña API real comprobó TERM/INT, reinicios y restauración de R1-R5. La aceptación API final y el CI de cierre aprobaron; la reevaluación se integró en `main` por PR 34, según el informe de verificación. La agenda conjunta se integró por PR 35. Las alertas durables, Qadra y su composición en servidor se integraron por PR 36 como `8261c51`, con aceptación propia registrada en el informe. Activación y corpus jurídico aplicable siguen pendientes. Los resultados se identifican por entrega; no sustituir el cómputo por fechas manuales ni una suma indiscriminada de días. |
| Tablero, informes y bitácora | Indicadores obtenidos de datos autorizados, filtros y exportaciones; consulta de auditoría separada de su verificación criptográfica. | Tablero operativo integrado por PR 45 con aceptación focal, navegador real y restauración; CI, Web y Documents de su confirmación natural en main aprobados. Informes durables integrados por PR49 en `f3bfed8`, con consumidor supervisado, captura cifrada común a PDF/CSV, avisos propios y selector autorizado que incluye membresías de casos cerrados; trece escenarios distintos de navegador controlado y dos con servicios reales aprobados, con descargas exactas tras nuevo ingreso; aceptación HTTP/restauración aprobada con reinicios y revocación completa; gates de PR49 y confirmación natural de main aprobados. Capacidad CSV máxima aprobada y PDF máximo rechazado de forma tipada; no equivale a generación satisfactoria a todos los máximos. Correo, estimación de terminación, otros tipos de informe y desempeño permanecen pendientes. Véase [el contrato](case-reports-api.md). La descarga exige acceso a toda la captura. La consulta filtrada Owner aprobó sus pruebas focales, API/restauración y dos escenarios reales; PR50 integrada en `a8dc3dd`, con confirmación natural de main aprobada. No incorpora aún identificador estable de cuenta ni IP; tampoco demuestra el límite de registro de 500 ms o anclaje externo. No presentar una página como total del despacho ni la captura actual como un censo histórico. |
| Validación integral | Casos positivos y negativos del catálogo completo, flujos reales desde navegador, rendimiento, fallos y recuperación; usabilidad con personal del despacho. | PostgreSQL y Redis aislados, TSA local, evidencias reproducibles, comparación visual de escritorio y móvil, métricas con entorno y fecha. Usabilidad requiere participantes reales y resultados observados. |

Las entregas se implementan en ramas `feat/` y se integran mediante PR y squash
con CI aprobado. Cada entrega funcional actualiza contrato, decisiones,
operación y apartados académicos afectados. El diseño aprobado se mantiene;
sus divergencias se resuelven explícitamente antes de marcar un objetivo como
cumplido. Las conclusiones y la presentación corresponden al cierre académico
posterior y no se completan a partir de un resultado parcial del backend.

## Fidelidad del diseño

La referencia visual es el sistema existente en `web/src/styles/`, los componentes
de `web/src/components/` y los originales con procedencia y licencia en
`web/public/brand/qadra/`. Se conservan marca, tipografía, colores, escala de
espaciado, navegación, controles, tarjetas y comportamiento adaptable a móvil.
Las nuevas vistas reutilizan estos elementos. Las ampliaciones se separan en
archivos para mantener el límite de tamaño de código.

La integración reemplaza las rutas documentales globales y las referencias
temporales por operaciones de la API de expedientes. Una verificación
criptográfica se solicita como acción explícita; no se usa para inferir el
detalle o la existencia de un documento. Al cambiar de expediente o sesión se
descartan respuestas anteriores y se evita mostrar datos del contexto previo.

## Matriz de cierre del catálogo funcional

Los estados describen el flujo completo de producto, no solo una primitiva o
una ruta. Cada cierre exige evidencia positiva y negativa, autorización,
persistencia, auditoría e interfaz aplicables. Los criterios se derivan del
catálogo versionado en análisis y diseño; no reemplazan objetivos aprobados.

| Caso de uso | Estado | Alcance y condición pendiente para cierre |
| --- | --- | --- |
| Registro de despacho y selección de plan | Parcial | Conservar bootstrap; conciliar selección comercial con instancia de un solo despacho y completar enrolamiento recuperable. |
| Ciclo de vida de miembros | Parcial; directorio y administración de acceso integrados | Directorio Owner, selección por correo, rol/estado con revisión esperada, protección del último Owner y revocación durable de sesiones/desafíos; evidencia focal, PostgreSQL/Redis, tres recorridos de navegador real y aceptación API completa con restauración aprobados y entrega integrada. La recuperación de correo/rol e intención incierta del alta está integrada por PR85, con aceptación local controlada y gates de la PR aprobados; no recupera contraseña ni MFA. Invitaciones y enrolamiento recuperable siguen pendientes. |
| Inicio de sesión y sesiones | Parcial; contraseña/MFA, recuperación pública y base de reingreso integradas | PR55–PR61 incorporaron políticas temporales, correcciones, reingreso y recuperación de contraseña con confirmación de main. Documentos, expedientes, participantes, audiencias/hechos y miembros/calendarios tienen recuperación integrada; PR64 confirmó main tras corregir el helper de preparación. PR66, PR69 y PR71–72 integraron recursos, asociaciones y plazos; PR74 y PR75 añadieron informes y preferencias con confirmación natural aprobada. PR77 integró confirmaciones administrativas y de sellado, con sus gates y confirmación natural aprobados. La lectura de avisos está integrada por PR81 con confirmación natural aprobada y las demás familias mantienen el estado del inventario de recuperación. Faltan duración operativa aprobada, remitente/origen/política del correo y cierre de las familias restantes. El dominio, verificador y aplicación del vínculo Owner están integrados por PR70, PR73 y PR78, con sus confirmaciones naturales aprobadas. PostgreSQL, HTTP compuesto e interfaz están integrados por PR82 y PR83, con confirmación natural aprobada y aceptación por capa y con servicios reales. El primer factor por certificado y su interfaz están integrados por PR84, con confirmación natural aprobada y aceptación RSA/MFA y restauración. La instalación y activación explícita siguen pendientes; estas opciones permanecen desactivadas en VPS3. La firma documental individual sigue pendiente. |
| Control de acceso por perfil | Parcial | Extender la matriz a cada módulo pendiente y comprobar el registro de accesos exigido por el catálogo. |
| Registro y administración de expediente penal | Implementado para el alta penal completa | NUC/carpeta y autoridades, delitos, metadatos, unicidad actual, Investigación inicial, edición y cierre con historia verificados. Las fichas anteriores se completan sin fabricar etapa; su adopción y las transiciones usan el recurso independiente de etapas. Los valores declarados no son certificaciones institucionales. |
| Directorio de participantes | Implementado para identidad representada y credencial interna de demostración | Once perfiles, datos declarados, identidad versionada, revisión explícita de coincidencias, unicidad de identidad/rol, soporte y firma interna; compatibilidad manual, consultas y estado auditados. Quedan fuera la acreditación civil/profesional, FIREL real y la certificación jurídica de expediente penal activo. El cierre organizativo bloquea mutaciones. |
| Transición de etapa procesal | Implementado para adopción y dos avances ordinarios | Perfil completo, documentos/versiones exactos, fechas declaradas, admisión PDF/DOCX, historia, conflictos, cierre y revocación tienen flujo persistente en Qadra. El registro no certifica la procedencia jurídica del acto ni implementa recursos o plazos. La programación de audiencias usa su propio historial. |
| Audiencia y activación de plazos | Parcial; programación y sesiones declaradas integradas | Programación, agenda de audiencias, resultados declarados, comparecencias, acuerdos, continuidad, rectificación y retiro tienen flujo propio. Sus campañas locales concluyeron y el código está integrado en `main`. La agenda combinada posterior se integró por PR 35. Las alertas se integraron por PR 36. Faltan activación consistente de plazos, catálogo restante y aceptación integral; registrar texto no calcula efectos jurídicos. |
| Calendario judicial | Parcial; catálogo con backend, API y Qadra verificados | Configuración global de cobertura, reglas y excepciones con fuentes declaradas, restauración, cobertura y recorridos de navegador aprobados; CI de calendarios aprobado. La selección exacta y la evaluación por perfil ya alimentan plazos persistentes; Qadra V1 de plazos tiene campañas locales completas aprobadas; V2 tiene recorridos API con restauración y navegador con backend real aprobados. La aceptación API final y el cierre global aprobaron; PR 34 integró la reevaluación en `main`. Faltan el cierre del corpus jurídico aplicable y la aceptación de los objetivos restantes del flujo completo; la actualización académica conserva su seguimiento propio. El conteo civil puro aporta una candidata; una URL o la clasificación de una fecha no acredita la regla normativa. |
| Monitoreo de plazos y alertas | Parcial; base V1 integrada y servicio/HTTP/Qadra V2 con recorridos reales aprobados | Registro, revisión exacta, responsable y atención tienen flujo web integrado. La ampliación humana/HTTP V2 declara políticas, conserva observaciones y continuidad y representa historia humana/técnica. Detalle actual y listado verifican vigencia sin usar la cola como prueba de actualidad. Despacho y consumo confirman trabajos, cursores, resultados e intentos con auditoría. Qadra V2 tiene pruebas Node y navegador con HTTP controlado y backend real aprobadas. La composición en servidor aprobó pruebas unitarias y una campaña API con TERM/INT, reinicios y restauración. La aceptación API final y el CI de cierre aprobaron; la reevaluación se integró en `main` por PR 34. La agenda conjunta se integró por PR 35. PR 36 integró alertas con persistencia, reintentos, control de duplicados, Qadra y composición en servidor como `8261c51`; su aceptación API/restauración, navegador real y cierre de CI conservan evidencia propia. La activación automática continúa pendiente. Los recorridos reales de reevaluación no completan esos objetivos restantes del caso de uso. |
| Carga y clasificación documental | Admisión general integrada por PR47; main confirmado | Ocho familias admitidas antes de cifrar nuevas cargas o versiones, hasta 16 MiB, con inspección acotada y decodificación multimedia completa. Clasificación atómica, filtros y versiones conservados; rechazo tipado sin borrar el borrador ni reenvío automático. Focales, API/restauración y navegador real aprobados; Clippy, PDF y provisión en los tres VPS aprobados; integrado por PR47 como `ac34b34`, con confirmación natural de main comprobada. Soportes procesales PDF/DOCX independientes e históricos sin readmisión. |
| Consulta e integridad documental | Parcial | Contenido sin sello y buzón interno Owner integrados con aceptación del incremento y cierre global aprobados. Los avisos describen fallos de validación, no causas demostradas. Historial, filtros de clasificación, verificación explícita y exportación de evidencia sellada implementados. |
| Firma de contrato y sello | Parcial | Vincular credencial y autorización al firmante individual y comprobar estado del certificado antes de firmar. |
| Verificación de firma y sello | Parcial | Política de identidad individual; la selección histórica explícita conserva los verificadores existentes. |
| Tablero de control | Indicadores integrados por PR 45; alcance jurídico parcial | Instantánea auditada Owner/Litigante, expedientes activos, contratos sin sello interno, plazos vigentes y carga autorizada; aceptación focal, navegador real y restauración aprobados. CI, Web y Documents aprobaron también para su merge en main. Firma personal pendiente y naturaleza fatal de plazos no se infieren de estos indicadores. |
| Informes | Estado/carga integrados por PR49; alcance completo parcial | Estado y carga de asignaciones, captura común cifrada PDF/CSV, selector autorizado, avisos internos y consumidor supervisado. Trece escenarios controlados y dos reales aprobados; cuatro capturas inspeccionadas y formatos exactos tras reingreso. Capacidad y presentación medidas por separado; aceptación HTTP/restauración aprobada con artefactos y avisos exactos y captura revocada; CI, Web y Documents de PR49 y su confirmación natural de main aprobados. Correo, estimación de terminación, demás tipos de reporte y desempeño conservan el alcance de CU-16/RF-19. |
| Consulta de actividad | Integrada por PR50 y confirmada en main | Owner global, filtros exactos, UTC con nanosegundos, páginas con máximo de secuencia fijo, preflight de texto y lectura auditada. Aplicación 20, PostgreSQL 14, HTTP 6, runtime 1, Node 12 y navegador controlado 9 aprobados. API/restauración exit0 en 376.459967 s y navegador real 2/2 en 13.7 s; Clippy aprobado. CU-17/RF-20 conservan identificador estable de cuenta, IP, cobertura uniforme de operaciones y registro menor de 500 ms. La verificación de la cadena es separada; el anclaje externo sigue pendiente. |

La validación final incluye usabilidad con participantes reales. Ningún resultado
de cobertura ni una demostración parcial cambia automáticamente estos estados.

## Estado del cronograma

El [plan de diez semanas](../latex/chapters/03-analisis-diseno.tex) reserva la
semana 9 para persistencia, autorización, gestión procesal/documental e interfaz,
y la semana 10 para aceptación final, usabilidad, correcciones y manuales.
El núcleo criptográfico y gran parte del producto están integrados, pero el
cierre funcional de la semana 9 continúa con los pendientes de esta matriz.
La infraestructura de CI y la release privada activa en VPS3 facilitan ese
trabajo; no cierran por sí mismas los módulos restantes.

La consulta de actividad avanza selección, persistencia e interfaz de la semana 9.
Aprobaron aplicación 20/20, PostgreSQL 14/14 en 37.01 s, HTTP 6/6, presupuesto
compartido 1/1, Node 12/12 y navegador controlado 9/9 en 17.1 s; las capturas
1440/390 fueron inspeccionadas y Clippy aprobó en 2m12s. La aceptación
API/restauración terminó con salida cero en 376.459967 s; el navegador real aprobó
2/2, Owner 6.2 s y Litigator 2.4 s, con 13.7 s de Playwright y 185.7532 s del
comando completo. En ambas campañas permanecieron idénticos 3198 archivos fuente.
Son duraciones de validación, no latencia por registro. PR50 y su confirmación
natural en `main` aprobaron, y la consulta está incluida en `v0.1.0`.
UUID estable, IP y medición del registro menor de 500 ms conservan el alcance
de CU-17/RF-20.

La semana 10 conserva pendientes la validación con participantes reales,
los manuales finales y la conciliación de resultados y conclusiones. Las
pruebas automáticas y la inspección visual ya disponibles aportan evidencia
técnica, pero no sustituyen el estudio de usabilidad. Los capítulos y los
objetivos aprobados se conservan; el cronograma se informa por entregas y
condiciones de aceptación, sin convertirlo en un porcentaje ni asignarle
fechas de cierre nuevas no acordadas.

| Pendiente de aceptación | Evidencia necesaria para cambiar su estado |
| --- | --- |
| Aceptación autenticada en VPS3 | Crear el primer Owner con el correo elegido por el operador y verificar los flujos de la release activa. La salud del servidor y la restauración inicial SQL/PKI con cero usuarios no acreditan ese recorrido. |
| Registro de auditoría en menos de 500 ms | Medición por operación y confirmación durable, con volumen de historial, concurrencia y host declarados; tiempos individuales y resumen de distribución. Un índice utilizable no acredita ese umbral. |
| Usabilidad con personal del despacho | Kit, protocolo y plantillas preparados; todavía sin evaluación humana. Participantes reales y tareas acordadas; registrar terminación, tiempo, errores, asistencia y observaciones, junto con las correcciones verificadas. El protocolo por sí solo sigue pendiente de ejecución. |
| Manuales y conclusiones | Conciliar instrucciones con una revisión reproducida; compilar e inspeccionar el manuscrito y redactar conclusiones a partir de los resultados completos. Los marcadores pendientes no equivalen a cierre académico. |
| Proveedor externo de sellado | La extensión posterior definida por el objetivo aprobado no se acredita mediante la TSA local. Si se ejecuta, identificar proveedor/entorno, evidencia recibida y verificación independiente con casos de aceptación/rechazo, tiempos y disponibilidad observados. No se declara ejecutada ni se redefine como requisito de esta consulta. |

La introducción, los objetivos, el estado del arte y las conclusiones pendientes
conservan su redacción. El incremento de consulta no modifica los criterios
aprobados ni produce un nuevo porcentaje global del TT.

## Conciliación de alcance

- El prototipo atiende un solo despacho. Registro y selección de planes deben
  conciliarse con esta delimitación antes de introducir facturación o aislamiento
  multiinquilino. Ninguna de esas ampliaciones es requisito implícito para
  conectar la interfaz existente.
- La autenticación integrada en `main` conserva contraseña con MFA y añade el
  certificado como primer factor alternativo para Owner, también con MFA
  obligatorio. Esta opción requiere activación explícita y sigue deshabilitada
  en VPS3. El sello documental usa
  la credencial configurada al arrancar el servidor. Las declaraciones internas
  de participantes verifican una firma externa de 384 bytes sobre una declaración
  de 218 bytes; no reciben la clave privada. Ese flujo no vincula automáticamente
  la identidad representada con una cuenta de acceso. La firma documental
  individual de usuarios continúa pendiente.
- El corte histórico de sesiones declaradas midió Argon2id en 409.6 ms sobre
  cinco corridas con dos iteraciones. La medición aislada posterior del mismo
  costo promedió 400.3851816 ms, también por debajo de 500--1000 ms. El ajuste
  actual usa tres iteraciones para hashes nuevos, conserva 256 MiB y un carril
  y aprobó siete pruebas focales, incluida la verificación de PHC históricos
  con dos iteraciones. La medición real de tres iteraciones promedió
  659.7596048 ms en cinco hashes y obtuvo veredicto dentro de 500--1000 ms:
  la banda queda comprobada para hashes nuevos en el host y binario medidos,
  no para el login completo ni HTTP bajo carga. Seis pruebas CLI aprobaron en
  20.916 s y la demostración completa en 8.240 s; esta midió por separado
  563.2 ms en cinco hashes, también dentro de banda, sin sustituir la medición
  principal. El incremento está integrado; la evidencia de PDF corresponde a su revisión específica. Véase
  [ADR-0044](adr/0044-reference-password-hashing-cost.md).
- Client mantiene acceso a metadatos de expedientes asignados y denegación
  documental. Ampliarlo requiere una política de recursos explícita y pruebas
  de aislamiento; ocultar botones no constituye autorización.
- El criterio OE-2 conserva su redacción aprobada sobre cuatro etapas. La
  [propuesta de recursos](procedural-resources-scope.md) distingue las tres etapas
  del CNPP y recursos vinculados a resoluciones, sin modificar ese criterio.
  Corregirlo literalmente requiere autorización explícita; implementar las dos
  transiciones no cierra recursos ni el cómputo automático exigido.
- El catálogo de programación distingue cuatro tipos de audiencia; el marco
  también menciona medidas cautelares y continuaciones. Las sesiones declaradas
  modelan continuidad mediante otra raíz con antecedente exacto preexistente,
  con comparecencias y acuerdos propios. Esto no amplía el catálogo de citas ni
  cubre las reglas específicas de medidas cautelares. CU-08 y RF-07 permanecen
  parciales: sus criterios aprobados incluyen activación de plazos y alertas,
  además de la captura implementada y la aceptación integral.
- CU-09 permanece parcial. El catálogo distingue fechas `countable`, `excluded`,
  `unresolved` y `outside_coverage` según el ámbito declarado. No adopta un
  calendario universal, no descarga normas ni asigna calendarios comparando el
  nombre de una autoridad. El registro explícito ya evalúa y conserva el
  resultado de fuentes y perfiles exactos. Los cambios durables se procesan en
  servidor y la agenda conjunta está integrada por PR 35.
  El flujo automático completo requiere cerrar perfiles jurídicos con fuentes
  primarias y corpus de aceptación, activar términos y comprobar su enlace con
  las alertas integradas y verificadas por separado en PR 36. La selección y calificación manuales implementadas no completan esos pendientes
  ni la aceptación integral de los demás casos del catálogo.
- Las alertas internas y la entrega por correo tienen contratos distintos. No se
  da por completada una notificación por persistir únicamente un vencimiento.
- El archivo de una ficha es organizativo. No prueba una transición jurídica,
  nombramiento o legitimación; una etiqueta procesal no modifica RBAC. La revisión
  de fuentes oficiales en ADR-0021 mantiene separados certificado, identidad y
  representación. La declaración de demostración de ADR-0026 comprueba posesión
  de una clave y vincula el contenido exacto registrado con confianza interna
  capturada. El criterio de FIREL del catálogo permanece fuera de esa equivalencia;
  no se presenta una cadena interna como credencial judicial externa.
- La TSA local y la CA interna permiten verificar evidencia técnica. La campaña
  con un PSC externo permanece fuera de la entrega actual, según
  [la decisión sobre TSA local](adr/0009-local-timestamp-authority.md).

## Validación y dependencias externas

Antes de cerrar, la matriz de los casos de uso debe enlazar cada comportamiento
con su prueba y resultado. La cobertura de líneas y las pruebas criptográficas
existentes no prueban por sí mismas el catálogo procesal ni la usabilidad.
Los ensayos de navegador con respuestas simuladas se registran separados de
aquellos que ejecutan Rust, PostgreSQL, Redis y TSA. Una captura o un render
verifica presentación; no demuestra por sí solo una operación confirmada.

El servidor requiere qpdf 12.4.1 en Linux x86_64 y comprueba su worker antes de
aceptar tráfico. La nueva admisión general añade FFmpeg 9.0.2 con componentes
restringidos, red deshabilitada y comprobación de ocho muestras al arrancar.
Preparación, variables y límites están en la
[operación del validador de soportes](document-format-operations.md) y el
[contrato de admisión general](document-upload-admission-api.md). La validación
técnica no acredita autenticidad jurídica ni detección de malware; sus límites
y protocolos no equivalen a un aislamiento completo del sistema operativo.

La usabilidad requiere acordar participantes, tareas y condiciones, obtener
observaciones reales, corregir los problemas detectados y registrar los
resultados. Preparar el protocolo o simular un usuario no completa ese ensayo.
Mientras falte la participación externa, continúan las tareas de implementación
y validación técnica que puedan ejecutarse independientemente.

El despliegue público requiere además resolver los límites operativos enumerados
en [la revisión del backend](backend-review.md), revisar el modelo de amenaza de
firma HTTP y delimitar la ausencia de anclaje externo de auditoría. La
demostración local no se presenta como validación de producción.

## Recursos procesales: checkpoint funcional publicado

El registro de revocación y apelación conserva una resolución histórica exacta,
su soporte y personas recurrentes, con fichas del directorio opcionales. Los
actos tienen identidad y revisiones propias; su corrección no sustituye evidencia
anterior. Archivo y reactivación organizativos permanecen separados del
acto de desistimiento. PostgreSQL mantiene escrituras y auditoría atómicas;
HTTP y Qadra exponen preparación, confirmación, consulta e historia autorizadas.

La verificación focal incluye dominio, aplicación, HTTP, doce pruebas PostgreSQL
con restauración y recorridos de navegador con HTTP controlado. La compilación
web está aprobada. La aceptación HTTP completa conservó veinte respuestas
exactas después de restaurar la base y los tres recorridos con servidor real
aprobaron escritorio, móvil y permisos. La PR 37 conserva ese checkpoint
histórico; el registro y su cierre posterior ya están integrados en `main`.
La revisión del PDF corresponde a la evidencia documental de cada entrega.

La ampliación de [asociaciones existentes](resource-activities-api.md) está
integrada en `main`: selecciona recurso y acto históricos, vincula una
audiencia o plazo exacto, consulta su estado actual por separado y conserva
historia al desvincular. Su persistencia y auditoría son atómicas; Qadra ofrece
selectores, revisión previa, filtros, historia y conflicto explícito. Las
pruebas focales de aplicación y PostgreSQL verifican que el instante consultado
se tome después del bloqueo y dentro de la ventana de lectura. La aceptación
API/restauración integrada aprobó en 434.402 s, con 26 respuestas restauradas
y sus instantes de lectura validados por separado. Tres escenarios distintos de navegador real
aprobaron; el incremento se integró posteriormente en main; se conserva separada la aceptación
previa de PR 37.

La [creación contextual de plazos](resource-deadlines-api.md) está integrada
como entrega posterior: prepara un plazo y su asociación, exige una
fuente temporal explícita y confirma ambos registros con un marcador auditado
de origen conjunto. La repetición exacta conserva ambos recibos; dos operaciones
ordinarias independientes no acreditan ese origen. La aceptación local y los tres gates aprobaron; PR46 está integrada. La
confirmación natural de main aprobó los tres gates y se registra separadamente.

La [lectura inversa desde actividades](activity-resource-links-api.md) quedó
integrada por PR48. El panel **Recursos relacionados** consulta asociaciones
por expediente y audiencia o plazo, filtra sus cabezas actuales y pagina sin
recorrer todos los recursos en el navegador. La observación común también
existe en una página vacía. Abrir una fila conserva el recurso y la asociación
exactos; el regreso mantiene la revisión de actividad original y, si procede,
la bandeja de alertas con sus filtros. Un rechazo retira los datos privados y
las respuestas tardías no restituyen capturas de un contexto cerrado.

La revisión abierta desde el aviso, la capturada por la asociación y la cabeza
actual se muestran por separado. Son vínculos actuales: no se afirma que
existieran al emitirse la alerta ni que fueran su causa. La consulta no marca
avisos como leídos, modifica recibos ni concede permisos por haber recibido
una alerta. Owner y personal asignado conservan las lecturas autorizadas;
Client sigue denegado. La verificación focal aprobó cinco casos de aplicación,
seis PostgreSQL distintos y doce regresiones previas, seis HTTP nuevos y diez
regresiones previas, ocho Node, nueve escenarios distintos de navegador con
HTTP controlado y nueve del planificador. Un recorrido real aprobó en 11.6 s
de escenario y 15.5 s de ejecutor; se inspeccionaron sus capturas a 1440 y
390 píxeles. La campaña API/restauración aprobó con salida cero, conservando
las capturas exactas y los filtros tras recuperar el respaldo. Clippy aprobó
y el PDF final de esa entrega, de 345 páginas, se compiló e inspeccionó. La entrega quedó
integrada por PR48 en `5020707`; CI, Web y Documents de la confirmación natural
aprobaron en 6m41s, 11m24s y 7m43s, con 3156 Rust, 396 escenarios controlados
y 47 reales, además de dos ignorados. Se conserva separada
la aceptación de esta entrega de la aceptación de informes integrada por PR49.

Persisten las audiencias propias de recursos, el corpus jurídico calificado y
la activación automática durable de sus términos. Asociar o consultar no crea
otro aviso ni cambia los destinatarios de la actividad. El [alcance completo](procedural-resources-scope.md),
el [contrato de registro](procedural-resources-api.md) y
[la evidencia por entrega](verification-report.md) conservan esos pendientes.
