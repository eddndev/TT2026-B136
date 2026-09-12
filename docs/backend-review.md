# Barrido del backend

Revisión histórica del 11 de septiembre de 2026 sobre la base `0cc6921`.
La autorización documental, las transacciones y el corte recuperable se abordan
en [ADR-0016](adr/0016-case-document-transactions.md). Consultar
[el informe vigente](verification-report.md) para el estado posterior.

## Plan de barrido

1. Contrastar ramas, cambios locales, puertos, adaptadores y contrato vigente.
2. Revisar autorización, invariantes persistidos y operaciones concurrentes;
   reproducir cada defecto antes de corregirlo.
3. Revisar límites de recursos, errores y claridad: separar responsabilidades,
   eliminar helpers duplicados y mantener las dependencias hacia el dominio.
4. Ejecutar pruebas con PostgreSQL y Redis reales, demostraciones y cobertura;
   registrar resultados actuales y entregar por PR con CI antes del squash.
5. Preparar el siguiente objetivo según dependencias y capacidad del cronograma.

## Hallazgos corregidos

| Debilidad | Corrección y evidencia |
| --- | --- |
| Dos factores podían completar el mismo desafío MFA. | Consumo atómico previo; pruebas con intentos solapados y factor rechazado. |
| Crear usuarios confiaba en un Principal entregado por el llamador. | Sesión y permiso vigentes en el caso de uso; pruebas de rol cambiado, cuenta inactiva y sesión revocada. |
| Contadores Redis podían perder su TTL. | Incremento y caducidad atómicos; la lectura repara bloqueos heredados sin TTL y conserva ventanas válidas. |
| Un segundo proceso podía sobrescribir un sello ya persistido. | Bloqueo por documento, comparación de estado actual y metadatos inmutables; cuatro escritores compiten y solo uno sella. |
| Leer la bitácora podía observar una línea parcial. | Bloqueo compartido durante la lectura; prueba con escritor detenido a mitad del registro. |
| Los adaptadores aplicaban migraciones sin coordinación común. | Inicialización única y serializada; prueba de arranque simultáneo en esquema vacío. |
| Deserializar admitía versión cero o más códigos de recuperación. | Constructores validados; se conservan formatos válidos y ocho posiciones, incluidas las consumidas. |
| Incrementar la versión máxima podía desbordarse. | Error explícito y prueba en el límite numérico. |
| HTTP podía acumular cuerpos y trabajo criptográfico sin límite. | Admisión antes del cuerpo y presupuesto de trabajo compartido, retenido aunque se cancele la petición. |
| Helpers HTTP duplicados y responsabilidades mezcladas. | Módulos de identidad/documentos, parser Bearer y ejecución comunes; pruebas de compatibilidad y rechazo de ambigüedad. |
| JSON de identidad aceptaba campos ajenos y cuerpos de 16 MiB. | Campos estrictos y límite de 16 KiB; documentos conservan 16 MiB. |
| Correos con NUL o DEL llegaban al almacenamiento. | Rechazo antes de normalizar y pruebas que impiden llegar a hash, búsqueda o inserción. |
| Redis podía esperar indefinidamente después del handshake. | Plazos de conexión TCP y E/S; prueba con servidor que retiene una respuesta. |

La decisión y los compromisos están en
[ADR-0015](adr/0015-backend-concurrency-and-invariants.md). Los resultados
reproducidos se registran en [el informe de verificación](verification-report.md).

## Límites que siguen impidiendo declarar preparación para producción

- Asociar documentos a expedientes y autorizar cada operación por recurso.
  Cliente sigue bloqueado; los permisos documentales del personal son globales.
- Confirmar documentos, asignaciones y auditoría bajo una transacción coherente.
  El alta inicial también puede persistir un usuario antes de fallar su auditoría;
  la recuperación del material de enrolamiento necesita un diseño explícito.
- Resolver continuidad de la cadena entre todos sus escritores, restauración,
  reintentos y migración de los archivos actuales sin perder evidencia sellada.
- Definir TLS, pooling, plazos de PostgreSQL y del handshake Redis, límites de
  conexiones y cuerpos lentos, apagado ordenado y capacidad del despliegue.
  Los límites HTTP nuevos no prueban esos aspectos operativos.
- Revisar la firma RSA expuesta por HTTP frente al supuesto histórico de uso
  solo por CLI de [ADR-0002](adr/0002-rsa-signing-crate-and-advisory.md).
  La admisión de solicitudes no corrige por sí sola un canal lateral criptográfico.
- Resolver el anclaje externo de auditoría descrito en
  [ADR-0007](adr/0007-audit-chain-anchoring.md) y completar la interfaz y gestión
  procesal. La TSA local sigue siendo evidencia técnica de demostración.

El siguiente alcance y sus criterios de cierre están en [next-goal.md](next-goal.md).
