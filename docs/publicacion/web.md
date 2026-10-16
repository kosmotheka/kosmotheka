# La web (kosmotheka.com)

Web estática hecha por Claude Design ([encargo](../design/encargo-web.md)), descargada el 02/10/2026 a `web/` en la raíz del repositorio. Sustituye a las [páginas provisionales](provisional-pages/). Solo hace falta un servidor que sirva ficheros: no hay PHP, ni compilación, ni nada de terceros.

```
web/
  index.html        elige el idioma (el guardado o el del navegador) y redirige a es/ o en/
  es/               index, preguntas, descargas, proyecto, privacidad, borrar-cuenta
  en/               index, faq, download, about, privacy, delete-account
  assets/           site.css, site.js, fuentes Figtree (con su licencia, `OFL.txt`), marca (SVG) y capturas (webp, claro y oscuro)
```

- La política de privacidad está en `es/privacidad.html` y `en/privacy.html`. Si cambia, se cambia a la vez en [politica-privacidad.md](politica-privacidad.md) y en las dos páginas, con la fecha de «Última actualización».
- Borrar la cuenta (lo exige Google Play a toda app con cuentas, 16/10/2026): `es/borrar-cuenta.html` y `en/delete-account.html`, hechas a partir de la de privacidad. URL para Play Console: `https://kosmotheka.com/es/borrar-cuenta.html`. Prometen borrar en 30 días como mucho lo que llegue por correo; si cambia lo que se borra al borrar la cuenta (D99), se cambian también.
- URL para Play Console: `https://kosmotheka.com/es/privacidad.html` (la provisional era `/privacidad.html`, que con la web nueva deja de existir).

## Subirla al servidor

La sube el maintainer desde su terminal. **Ningún agente entra en el servidor de producción.**

Las variables (se ponen una vez en la terminal):

```bash
WEB_HOST=usuario@servidor          # el acceso ssh
WEB_DIR=/ruta/de/la/web            # la raíz del dominio en el servidor, sin barra final
```

### 1. Copia de lo que hay ahora

Por si hay que volver atrás. Va fuera del repositorio:

```bash
rsync -azh "$WEB_HOST:$WEB_DIR/" ~/kosmotheka-web-copias/$(date +%Y-%m-%d_%H%M)/
```

### 2. Ensayo

`-n` no toca nada: lista lo que subiría y lo que borraría (`*deleting`). Revisar que no borre nada que no sea de la web.

```bash
cd ~/Proyectos/personales/library
rsync -azhn --delete --delay-updates --itemize-changes \
  --chmod=D755,F644 --chown=www-data:www-data \
  --exclude='.well-known/' \
  web/ "$WEB_HOST:$WEB_DIR/"
```

### 3. De verdad

El mismo comando sin `n`:

```bash
rsync -azh --delete --delay-updates --itemize-changes \
  --chmod=D755,F644 --chown=www-data:www-data \
  --exclude='.well-known/' \
  web/ "$WEB_HOST:$WEB_DIR/"
```

Qué hace cada cosa:
- `-a` copia todo conservando fechas (incluye `-r`), `-z` comprime, `-h` muestra tamaños legibles.
- `web/` con barra final: sube el contenido de la carpeta, no la carpeta. Ojo con la ruta de destino: una errata (por ejemplo, `/var/www` repetido) crea una carpeta nueva y la web no cambia.
- `--delete`: borra en el servidor lo que ya no está en `web/` (en la primera subida, las páginas provisionales). No hace falta borrar nada a mano antes, y la web no queda caída mientras sube.
- `--delay-updates`: sube todo a ficheros temporales y los cambia de golpe al final, para que nadie vea una página nueva con estilos viejos.
- `--chmod=D755,F644 --chown=www-data:www-data`: permisos de lectura y el usuario del servidor web. Sin `--chown`, `-a` conserva el usuario local (el 1000) cuando en el servidor se entra como root.
- `--exclude='.well-known/'`: no toca la carpeta que usa Let's Encrypt para renovar el certificado (ni lo que se ponga ahí, como `assetlinks.json` para Android). Si en el servidor hay algo más que no es de la web, se añade otro `--exclude`.

### 4. Comprobar

Abrir `https://kosmotheka.com/`, cambiar de idioma y de tema, y abrir la política de privacidad en los dos idiomas.

Para volver atrás: el mismo comando del paso 3 con la copia como origen (`~/kosmotheka-web-copias/<fecha>/` en lugar de `web/`).
