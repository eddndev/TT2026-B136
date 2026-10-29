
## 5. Manifiesto sellado

`manifiesto.json` enumera en JSON canónico (RFC 8785) cada archivo cubierto de
este paquete con su función, su tamaño y su huella SHA-256, además de la cuenta
y el vínculo del titular. Su huella SHA-256 es `{{MANIFIESTO_DIGEST}}` y
`manifiesto.json.tsr` es un sello de tiempo sobre ella:

    openssl dgst -sha256 manifiesto.json
    openssl ts -verify -data manifiesto.json -in manifiesto.json.tsr -CAfile {{ANCLA_SELLO}}
    openssl ts -reply -in manifiesto.json.tsr -text

Compare la huella de cada archivo con la que declara el manifiesto. El sello
del manifiesto no puede ser anterior al sello del contenido.

`tsa-crl.pem` es la lista de revocación del emisor del certificado de la TSA,
capturada al obtener el sello del manifiesto. Debe cubrir el tiempo de
generación de ese sello y no listar el certificado de la TSA:

    openssl crl -in tsa-crl.pem -noout -text

`recibo-aceptacion.json` repite la huella del manifiesto y el instante de
aceptación registrado por la aplicación ({{ACEPTACION_AT}}). Ningún sello de
tiempo cubre el recibo, `individual-signing.json` ni estas instrucciones, y el
manifiesto no los enumera.
