# Informe de verificación local

## Corte reproducido

- Fecha local: 17 de agosto de 2026 (`America/Mexico_City`).
- Fecha observada en la salida UTC de la demostración: 17 de agosto de 2026.
- Revisión verificada: cierre de la rama `docs/avance-cripto-beamer` antes de
  versionar los cambios.
- Rust: `rustc 1.94.0` y `cargo 1.94.0`.
- OpenSSL: `3.5.7` del 9 de junio de 2026.
- Cobertura: `cargo-llvm-cov 0.8.7`.

## Suite automatizada

Comando:

```bash
cargo test --workspace
```

Resultado:

```text
367 funciones de prueba descubiertas
366 aprobadas
0 fallidas
1 ignorada
```

La prueba ignorada es el humo contra el sandbox real del proveedor de
sellado. Se conserva para documentar el adaptador, pero no se ejecuta como
requisito de esta entrega: la API es inestable y el sandbox es de pago con
precios no transparentes. Los otros 13 casos del
adaptador remoto se ejecutaron contra el stub HTTP local y aprobaron,
incluidos token inmediato, procesamiento diferido, rechazo, errores HTTP y
redacción de la credencial.

## Cobertura

Comandos:

```bash
cargo llvm-cov --workspace --json --summary-only \
  --output-path /tmp/tt2026-coverage.json
bash scripts/coverage-gate.sh /tmp/tt2026-coverage.json
```

Resultado:

```text
domain             934/  974 lines   95%  (gate: >=90%)
application       1137/ 1182 lines   96%  (gate: >=90%)
infrastructure    1881/ 2012 lines   93%  (gate: >=90%)
bin                833/  946 lines   88%  (gate: none)
```

Las proporciones sin truncar son 95.9 %, 96.2 %, 93.5 % y 88.1 %,
respectivamente. El total del workspace es 4 808 de 5 137 líneas, 93.6 %.

## Demostración integral

Comando:

```bash
bash scripts/demo.sh
```

La ejecución terminó con código cero y reprodujo:

1. Creación de la CA interna y emisión del certificado del firmante.
2. Emisión de un certificado de TSA y de un token RFC 3161 local.
3. Hash SHA-256 del documento de muestra.
4. Cifrado AES-256-GCM, alteración de un byte y rechazo autenticado.
5. Rotación de la KEK sin volver a cifrar el documento.
6. Firma RSA-3072 y verificación integral de cuatro componentes.
7. Alteración del documento y rechazo con causas por componente.
8. Exportación del ZIP de evidencia y verificación con OpenSSL y `unzip`.
9. Revocación del certificado y detección mediante CRL.
10. Calibración Argon2id, TOTP y bitácora encadenada con detección de cambios.

La verificación independiente produjo `Verified OK`, `certificado.pem: OK` y
`Verification: OK`. Los archivos temporales y secretos de demostración fueron
eliminados automáticamente al terminar.

## Decisión sobre el proveedor de sellado

El registro en el entorno de Cincel pudo completarse, pero eso no garantiza la
operación del servicio. Durante el consumo, la API no ofreció respuestas y
estabilidad suficientes para una campaña repetible; además, el sandbox
requiere pago y sus precios no son transparentes para presupuestar la muestra.
RTC-01 se registra como incidencia materializada; su probabilidad y VME se
conservan como línea base, no como medición de un costo ya incurrido.

Por decisión del proyecto, Cincel no es una dependencia técnica ni un criterio
de evidencia de la entrega actual. El adaptador y su stub se conservan para
demostrar la intercambiabilidad del puerto, pero la prueba contra el servicio
real permanece ignorada y no se agenda como campaña pendiente.

La decisión está registrada en
[`docs/adr/0009-local-timestamp-authority.md`](adr/0009-local-timestamp-authority.md).
La mitigación activa es `LocalOpensslTsa`, una TSA RFC 3161 cuyo certificado
RSA-3072 es emitido por la CA interna con `extendedKeyUsage = timeStamping`.
Opera detrás del mismo puerto que el adaptador remoto y sus tokens son
aceptados por `openssl ts -verify`. Esta vía demuestra continuidad técnica;
no aporta independencia de tercero ni sustituye una constancia NOM-151 emitida
por un PSC autorizado.

## Límites abiertos

- La evidencia externa de un PSC autorizado queda fuera del alcance de esta
  entrega. Una integración futura requerirá elegir un proveedor con contrato,
  disponibilidad y precios verificables; no se presenta la TSA local como
  sustituto jurídico de una constancia NOM-151.
- Argon2id quedó calibrado en el hardware de referencia (AMD Ryzen 7 7730U,
  16 hilos lógicos) con `m=262144,t=4,p=1`: cinco corridas promediaron 607.8 ms,
  dentro de la banda objetivo de 500 a 1 000 ms. Si el hardware de despliegue
  difiere, la medición debe repetirse.
- La API ya tiene un primer adaptador HTTP con `/healthz`; las sesiones, el
  control de acceso, la persistencia y la UI siguen siendo el trabajo inmediato.

La transcripción extensa de una corrida anterior se conserva en
[`docs/demo-transcript.md`](demo-transcript.md).
