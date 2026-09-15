# Consultas documentales autorizadas y proyección de metadatos

## Context

El flujo de documentos descrito en
[la transacción por expediente](0016-case-document-transactions.md) dispone de
carga, sellado, verificación y evidencia, pero una interfaz persistente necesita
listar documentos y obtener su detalle sin efectuar operaciones criptográficas.
Inferir la existencia o el estado de un documento a partir de una verificación
produce resultados ambiguos y trabajo innecesario. Los metadatos también son
información del expediente y requieren aislamiento y auditoría.

## Decision

Se añade el permiso de lectura documental para Owner, Litigator y Paralegal.
Client conserva su denegación documental. Listado y detalle reciben una sesión
en la aplicación y revalidan cuenta activa, rol, expediente y pertenencia en la
transacción PostgreSQL. El detalle exige además la asociación exacta entre
documento y expediente. Ausencia y falta de pertenencia tienen la misma
respuesta de recurso no encontrado.

Las consultas proyectan UUID, versión, nombre, digest e indicador de evidencia
presente. No recuperan el contenido cifrado ni el cuerpo de la evidencia. El
indicador de sellado no constituye una nueva comprobación criptográfica.

El listado admite búsqueda literal por nombre y filtro de sellado antes de
paginar, en orden UUID ascendente. La página contiene hasta cien elementos y
un indicador de existencia de otra página, calculado con un elemento adicional.
El desplazamiento permite paginar sin inventar un total global. La búsqueda
ignora diferencias entre mayúsculas y minúsculas para los nombres ASCII que
acepta el flujo documental; no descifra contenido ni indexa texto sensible.

Cada consulta usa la frontera auditada existente y confirma su evento antes de
devolver metadatos. Listado registra el expediente; detalle registra también el
documento. Los términos de búsqueda no se guardan en la bitácora. Los bloqueos
de identidad y asignación permanecen hasta el commit, por lo que una revocación
posterior espera y una revocación ya confirmada impide la lectura. Las respuestas
confirmadas pueden terminar de transmitirse después de una revocación.

La interfaz Qadra conserva su sistema de diseño y reemplaza las referencias de
sesión por consultas de la API en el expediente seleccionado. Al cambiar de
expediente o sesión se descartan resultados pendientes del contexto anterior.

## Status

Aceptada para consultas de la versión documental actual. El diseño de múltiples
versiones y clasificación requiere una decisión adicional que preserve la
evidencia histórica. La validación ejecutada se mantiene en
[el informe de verificación](../verification-report.md).

## Consequences

- La interfaz puede recuperar metadatos persistidos sin solicitar firma,
  descifrado ni verificación integral.
- Un fallo de auditoría impide entregar resultados, también para una lectura.
- Se reutiliza la serialización de la cadena de auditoría. Esto limita la
  concurrencia de consultas y exige medir su costo antes de ampliar la carga.
- La paginación por desplazamiento no mantiene una fotografía entre peticiones:
  inserciones concurrentes pueden desplazar elementos. El orden es estable para
  un conjunto de datos sin modificaciones.
- No se expone un total del despacho ni se indexa el contenido cifrado. El
  costo de búsquedas y desplazamientos debe medirse con el tamaño de datos del
  entorno de validación; esta entrega no establece capacidad de producción.
- La consulta de metadatos y la verificación criptográfica continúan siendo
  operaciones distintas, visibles como tales en la interfaz y el contrato.
