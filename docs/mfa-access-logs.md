# Registros internos de acceso MFA

Los POST a `/api/v1/auth/mfa/totp` y `/api/v1/auth/mfa/recovery` emiten un evento
`MFA access attempt` del target `qadra::access`, a nivel `info`. El servidor genera
un UUID por petición y lo devuelve en `X-Request-Id`; ignora el identificador
enviado por el cliente. El rechazo de autenticación conserva HTTP 401 y
`mfa_rejected`, sin motivos internos en el cuerpo ni en las cabeceras.

Cada evento contiene `timestamp_unix_ms` (hora UTC del servidor al resolver el
intento), `request_id`, `method` (`totp` o `recovery`), `user_id` (UUID obtenido
del desafío del servidor o `unavailable`), `result` y `reason`. No contiene
correo, cuerpo de petición, códigos, claves, contraseñas, tokens ni mensajes
de error procedentes de los adaptadores.

| Motivo | Interpretación |
| --- | --- |
| `accepted` | Sesión admitida y auditoría completada. |
| `invalid_code_or_outside_window` | TOTP incorrecto o fuera de ventana; el verificador no distingue ambas causas. |
| `code_already_used` | TOTP válido cuyo reclamo atómico de uso fue rechazado. |
| `challenge_expired_consumed_or_unknown` | No hay desafío disponible; el almacén no diferencia vencimiento, consumo previo o inexistencia. No se puede recuperar el usuario. |
| `account_unavailable` | El desafío identifica una cuenta que ya no se encuentra. |
| `account_inactive` | Cuenta inactiva al comprobar su estado. |
| `credentials_changed` | Generación de autenticación diferente o fuera del rango admitido, o identidad distinta antes de emitir sesión. Incluye cambios de rol; no identifica qué credencial cambió. |
| `recovery_invalid_or_used` | Código de recuperación inválido o consumido; no se distinguen ambas causas. |
| `certificate_authority_invalid_or_expired` | La procedencia, autoridad o vigencia del certificado no permite continuar; motivo agrupado. |
| `session_admission_rejected` | Falló una comprobación posterior de admisión de sesión. Esa interfaz agrupa cambios de cuenta, autoridad o estado de sesión y no permite distinguirlos; no se atribuye a un código incorrecto. |
| `rejection_unspecified` | Implementación del puerto que no proporciona diagnóstico detallado. |
| `operational_error` | Fallo interno; no se registra el texto del error. |
| `invalid_request` | Rechazo HTTP previo al caso de uso, como JSON inválido o cuerpo excesivo. |
| `request_capacity_exceeded` | No se pudo admitir la petición por el límite HTTP. |
| `outcome_unavailable` | No llegó un resultado del caso de uso al registro. |

`result` es `accepted`, `rejected` o `error`. Un fallo de almacenamiento o auditoría
es `error`; no se informa como código inválido. Se conserva el orden de validación:
el desafío se consume antes de comprobar el segundo factor y no se vuelve a
consultar para averiguar su historia. Si varias condiciones fallan, se registra
la primera observada. No se modifican las ventanas TOTP ni la protección de replay.

## Consulta en el servidor

El binario escribe en stderr y systemd lo recoge en el journal de `qadra-api`.
Con acceso autorizado al usuario de ejecución de la instalación documentada:

```bash
ssh -l qadra vps3 'journalctl --user -u qadra-api --since "15 minutes ago" --no-pager -o cat --grep "MFA access attempt"'
ssh -l qadra vps3 'journalctl --user -u qadra-api --no-pager -o cat --grep "UUID_DE_LA_PETICION"'
```

Sustituir `UUID_DE_LA_PETICION` por `X-Request-Id` obtenido en la respuesta. El
alias SSH por sí solo no concede lectura del journal de otro usuario. Para otra
instalación, usar la cuenta y unidad correspondientes; véase
[despliegue](deployment.md). Conservar el target a nivel `info`: por ejemplo,
`RUST_LOG=info,qadra::access=info`. Un filtro que suprima ese target impide ver
los eventos. Aplicar cambios de entorno mediante el procedimiento de despliegue.

Estos registros son diagnósticos operativos, separados de la cadena de auditoría
transaccional. Su retención y acceso dependen del journal del servidor. No se
garantiza persistencia ante terminación abrupta, fallo del destino de logs o
peticiones que no lleguen a la API. Si el trabajador ya comenzó, emite su resultado
aunque el cliente se desconecte. El identificador no concede acceso a la cuenta.
La función requiere desplegar una versión que incluya este cambio.
