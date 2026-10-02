# 0059. Lectura inversa de asociaciones por actividad

## Estado

Aceptado. El contrato y su evidencia se conservan en
`docs/activity-resource-links-api.md` y `docs/verification-report.md`.

## Contexto

Una alerta conserva la revisión exacta de la audiencia o plazo que la originó.
Las asociaciones de recursos también conservan capturas exactas de sus extremos,
pero pueden registrarse o retirarse después de esa alerta. La consulta existente
parte de un recurso y no permite descubrir los recursos asociados desde la
actividad. Recorrer todos los recursos en el navegador impediría paginar de
forma fiable y multiplicaría las lecturas privadas.

## Decisión

Se añade una lectura inversa por expediente y actividad tipada. La autorización,
la existencia del destino, las cabezas de asociaciones, la verificación de sus
capturas y la auditoría de lectura se resuelven en una transacción. Incluso una
página vacía incluye su instante de observación. El servicio comprueba el
contrato del puerto y reautentica al principal completo antes de devolver datos.

Cada página usa el estado vigente de las asociaciones, ordenadas por UUID,
con un límite de 1 a 100. El filtro de estado se aplica antes del límite; la
continuación es exclusiva. Cada petición conserva su expediente, destino y
filtro. Las páginas sucesivas son observaciones actuales independientes, no
una instantánea histórica retenida entre peticiones. No se modifican recibos,
extremos, estados de lectura de alertas ni actividades para hacer esta consulta.

La interfaz muestra por separado la revisión abierta de la actividad y la
revisión capturada por la asociación. Abrir un vínculo conduce al recurso y a
la asociación exactos, con regreso a la actividad original. Si el recorrido
comenzó en una alerta, conserva el regreso a su bandeja y filtros. Consultar la
cabeza actual del recurso sigue siendo una acción explícita distinta.

Los resultados tardíos se descartan cuando cambia el contexto o la sesión. Un
rechazo de acceso retira los datos privados; un error no se presenta como una
lista vacía. Owner y personal asignado conservan sus permisos de lectura;
Client permanece denegado. El acceso a una alerta personal no sustituye la
autorización de cada consulta posterior del expediente.

## Consecuencias

El usuario puede descubrir y recorrer asociaciones desde una alerta, audiencia
o plazo sin conocer UUID ni buscar en todos los recursos. El historial conserva
las revisiones y los recibos que ya existían. El panel describe asociaciones
actuales; no afirma que existieran al emitirse la alerta ni que la causaran.

Esta navegación no crea nuevas alertas, no marca avisos como leídos, no determina
efectos jurídicos y no activa automáticamente plazos. La calificación de reglas,
las audiencias propias de recursos y otros pendientes mantienen sus contratos
y aceptaciones independientes.
