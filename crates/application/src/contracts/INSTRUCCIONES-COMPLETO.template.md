## 5. Declaración, vinculación y certificado del despacho

Repita los pasos 1 a 3 con los archivos de la posición 2:

```
openssl x509 -in parte-2-certificado.pem -pubkey -noout > clave-publica-2.pem
openssl dgst -sha256 -verify clave-publica-2.pem \
  -signature parte-2-declaracion.sig parte-2-declaracion.bin
openssl dgst -sha256 -verify clave-publica-2.pem \
  -signature parte-2-vinculo.sig parte-2-vinculo.bin
openssl verify -CAfile ca.pem -crl_check -CRLfile parte-2-crl.pem parte-2-certificado.pem
openssl dgst -sha256 parte-2-vinculo-raiz.der
dd if=parte-2-declaracion.bin bs=1 skip=206 count=32 | xxd -p -c 32
dd if=parte-2-declaracion.bin bs=1 skip=174 count=32 | xxd -p -c 32
```

Las firmas deben devolver `Verified OK`, el certificado
`parte-2-certificado.pem: OK`, la huella de la raíz debe coincidir con la de
`ca.pem` y los dos últimos valores con los SHA-256 de `contrato.pdf` y de
`contrato-solicitud.bin`.

## 6. La contrafirma cubre la firma y el sello del Cliente

Los 32 bytes desde el desplazamiento 239 de `parte-2-declaracion.bin` son el
SHA-256 de `parte-1-declaracion.sig`, y los 32 desde el 271 el de
`parte-1-declaracion.sig.tsr`:

```
dd if=parte-2-declaracion.bin bs=1 skip=239 count=32 | xxd -p -c 32
dd if=parte-2-declaracion.bin bs=1 skip=271 count=32 | xxd -p -c 32
sha256sum parte-1-declaracion.sig parte-1-declaracion.sig.tsr
```

Las dos primeras líneas deben coincidir, en ese orden, con las dos huellas de
la última orden: la firma del despacho declara qué firma y qué sello del
Cliente vio.

## 7. Sello del PDF (Td) y del manifiesto (Tf)

```
openssl ts -verify -in contrato.pdf.tsr -data contrato.pdf \
  -CAfile ca.pem -untrusted tsa-chain.pem
openssl ts -verify -in contrato-manifiesto.json.tsr -data contrato-manifiesto.json \
  -CAfile ca.pem -untrusted tsa-chain.pem
```

`contrato-manifiesto.json` lista el SHA-256 y el tamaño de cada entrada del
paquete salvo él mismo, su sello, `contrato-recibo.json` y esta guía.
Compárelos con `sha256sum *`. Tf demuestra que ambas declaraciones y firmas,
T1, Td, los certificados y las CRL existían a esa hora; no demuestra que nadie
leyera el contrato. Los sellos deben cumplir T1 <= Td <= Tf y Tf, con su
exactitud, debe ser anterior al vencimiento firmado.

