# Lo que sirve `kosmotheka.app`

`kosmotheka.com` es la web ([web.md](../web.md)). `kosmotheka.app` es el dominio de los **enlaces de invitación** (D79): `https://kosmotheka.app/i/<código>`. Todo lo demás de ese dominio redirige a la web.

- `invitacion.html`: lo que ve quien abre un enlace de invitación en un navegador. Explica que el enlace se pega dentro de la app, lo enseña y lo deja copiar. Un solo fichero, sin nada externo: ni fuentes, ni imágenes, ni peticiones a ningún sitio. Elige español o inglés por el idioma del navegador; sin JavaScript se ven los dos textos.
- `.well-known/assetlinks.json`: el fichero con el que Android comprueba que el dominio es de la app, para que el enlace abra la app sola (7.6.4). **Es una plantilla**: lleva dos huecos `PONER-AQUI-…` para las huellas SHA-256 de la firma de Google Play y de la clave de subida. No se sube hasta rellenarlos; de dónde sale cada huella y cómo se comprueba en un móvil, en [firma-android.md](../firma-android.md#4-enlaces-verificados-las-huellas-de-assetlinksjson-764). Debe servirse con tipo `application/json`, respuesta 200 y **sin redirección** (Android no la sigue).

## Reglas para el servidor web

El código de la invitación va en la dirección, así que la página se sirve con cuidado:

- **Sin registro de accesos** para `/i/`.
- **Sin redirección** de `/i/` a ningún otro dominio: el código acabaría escrito en otro registro.
- Cabeceras `Referrer-Policy: no-referrer`, `Cache-Control: no-store` y `X-Robots-Tag: noindex`.
- La misma página para cualquier `/i/<lo que sea>`: no sabe si el código vale; eso lo dice la app.

Con nginx:

```nginx
location ^~ /i/ {
    access_log off;
    add_header Referrer-Policy no-referrer always;
    add_header Cache-Control no-store always;
    add_header X-Robots-Tag noindex always;
    try_files /invitacion.html =404;
}
location ^~ /.well-known/ {
    default_type application/json;
    try_files $uri =404;
}
location / { return 301 https://kosmotheka.com$request_uri; }
```

Con el enlace ya abriendo la app sola (7.6.4), esta página sigue sirviendo para quien no la tenga instalada o lo abra en un ordenador.
