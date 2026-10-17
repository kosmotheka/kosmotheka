# IA para reconocer imágenes: Symfony AI y lo que haría falta

> Nota de investigación del 17/10/2026, sin código. Prepara D20 (abierta). **El maintainer decidió ese día el orden:**
> 1. Primero la foto de balda **sin salir del dispositivo** (D19, vía 3, con el lector de texto de ML Kit).
> 2. La IA, al roadmap, **una vez la app esté en producción**.
>
> Las cifras de precio son de la fecha de la nota: hay que mirarlas otra vez al empezar.

## Qué es Symfony AI

Un conjunto de componentes de Symfony (`symfony/ai-*`) que da **una sola interfaz para hablar con muchos proveedores de IA**:
- **Platform:** mandar mensajes, con imágenes, a un modelo y leer la respuesta. Es la que nos interesa.
- **Agent**, **Chat**, **Store** y otros: no hacen falta para esto.
- **AI Bundle:** lo integra en Symfony con configuración.

Cada proveedor es un paquete aparte, que se instala solo si se usa (comprobado en Packagist el 17/10/2026):

| Paquete | Para |
|---|---|
| `symfony/ai-anthropic-platform` | Claude (Haiku, Sonnet…) |
| `symfony/ai-open-ai-platform` | OpenAI |
| `symfony/ai-generic-platform` | **Cualquier servicio con el formato de OpenAI**: OpenRouter, LiteLLM, LM Studio, el servidor de llama.cpp… |
| `symfony/ai-ollama-platform`, `-lm-studio-platform` | Modelos en local, con nombre propio |
| `-gemini-`, `-mistral-`, `-open-router-` y unos 30 más | Otros proveedores |

**Cómo se configura** (de la documentación del bundle):

```yaml
# config/packages/ai.yaml
ai:
    platform:
        anthropic:
            api_key: '%env(ANTHROPIC_API_KEY)%'
        generic:
            local:
                base_url: '%env(AI_BASE_URL)%'
                api_key: '%env(AI_API_KEY)%'
```

En el código, el mismo `PlatformInterface` sirve para todos:

```php
$result = $platform->invoke($model, new MessageBag(Message::ofUser($prompt, Image::fromFile($path))));
```

Cambiar de Claude a OpenAI o a un modelo local es cambiar la configuración. Admite imágenes en el mensaje (`Image`) y pedir la respuesta con una estructura fija («salida estructurada»): una foto entra y sale una lista de títulos con su editorial y su autoría.

**Esto resuelve lo que preocupaba** («conectar con todas es imposible»): no hay que escribir un adaptador por proveedor. Quien monte su servidor elige el suyo en `ai.yaml`.

## El problema: es experimental

- **Versión 0.14.1** (26/09/2026). Saca una versión menor casi cada mes: 0.10 en junio, 0.11 y 0.12 en julio, 0.13 en agosto, 0.14 en septiembre.
- **El README del componente Platform lo dice:** «This Component is experimental». No está cubierto por la promesa de compatibilidad de Symfony. Una versión menor puede romper lo que se escribió.
- Pide PHP 8.2 o más y Symfony 7.3 u 8: con lo nuestro (PHP 8.5, Symfony 8.1) encaja.
- Se usa mucho: más de dos millones de descargas de `ai-platform`.

**Recomendación:**
- Usarlo, pero con la versión fijada (`0.14.*`, no `^0.14`).
- Todo detrás de una interfaz nuestra pequeña, por ejemplo `ShelfReader::read(imagen): lista de lomos`, como los proveedores de metadatos detrás de sus `*Source`. Si una versión rompe algo, se arregla en una sola clase.
- **Al empezar, comprobar si ya ha salido la 1.0**: entonces este punto desaparece.

## Dónde iría la IA: servidor o app

| | En el servidor (Symfony AI) | En la app (Rust) |
|---|---|---|
| Quién la tiene | Quien tiene cuenta, como TMDB o BoardGameGeek hoy | Cualquiera que ponga su clave, sin cuenta |
| La clave | La del servidor, con su límite por cuenta y su tope diario (como `bgg_account` y `BGG_DAILY_CALL_CAP`) | La de cada persona, en Ajustes › Proveedores |
| Varios proveedores | Symfony AI, configurado | Habría que escribirlos. D20 proponía dos adaptadores: Anthropic y compatible con OpenAI (este último cubre OpenAI, OpenRouter y los modelos locales). **Por mirar:** si hay una crate de Rust que haga lo que Symfony AI (`genai`, por ejemplo) |
| Por dónde pasa la foto | Móvil → nuestro servidor → proveedor | Móvil → proveedor |

**Recomendación:**
- **Empezar solo por el servidor.** Es el mismo camino que ya tienen los demás proveedores «a través del servidor», con sus límites, su caché y su aviso.
- La clave propia en la app, después y solo si alguien la pide.
- El servidor **no guarda la foto**: la manda y la olvida. Igual que hoy no guarda el texto de una búsqueda.

## Cuánto cuesta

Para una foto de un tramo de balda, con 15 a 20 lomos (lo que salió de la [prueba del 12/10/2026](foto-estanteria.md)):
- **La imagen:** Anthropic la cuenta como tokens, más o menos ancho × alto / 750. Reducida a unos 1.500 píxeles por el lado largo, son **unos 1.600 tokens de entrada**.
- **La respuesta:** 20 títulos con editorial y autoría son **unos 600 tokens de salida**.

| Modelo | Precio por millón de tokens (entrada / salida) | Por foto, aproximado |
|---|---|---|
| Claude Haiku 4.5 | 1 $ / 5 $ | **medio céntimo** (0,005 $) |
| OpenAI, modelo pequeño con visión | **comprobar al empezar** | del mismo orden |
| Modelo local (Ollama, LM Studio) | 0 | 0 $, pero va lento sin GPU y lee peor que un modelo grande |

Una biblioteca de 1.000 libros, con unas 60 fotos, saldría por **unos 30 céntimos con Haiku**. El coste no es el problema: lo son el tope por cuenta y que la foto salga del dispositivo.

## Lo que cambia fuera del código

La foto sale del dispositivo, y eso toca todo lo que dice qué sale:
- **La política de privacidad:** una sección «Si lees una estantería con la cuenta», como las de los proveedores. Contaría qué se envía (la foto), a quién (el proveedor que configure el servidor) y que no se guarda.
- **Seguridad de los datos de Play** ([play-console-formularios.md](../publicacion/play-console-formularios.md), sección 8): entra **«Fotos y vídeos › Fotos»**, recogido, efímero, opcional y para la funcionalidad de la app. Quizá también «compartido», si el proveedor no cuenta como encargado: hay que mirarlo con sus condiciones de datos.
- **La bienvenida y las Novedades:** una línea con su interruptor (regla de `whats-new.json`), y una entrada en Ajustes › Proveedores para apagarlo.
- **Las condiciones del proveedor:** que no entrene con las fotos. La API de Anthropic no entrena con lo que recibe por defecto; para OpenAI, comprobarlo.

## Cómo encaja con D19

- **Las mismas piezas sirven con y sin IA.** Primero la cola de revisión: la foto da una lista de lomos leídos, cada uno se busca en los proveedores, la persona confirma y elige la edición. Nunca se da de alta nada sin confirmar.
- **Primero sin IA (decisión del 17/10/2026):** el lector de texto de ML Kit en el móvil (vía 3) da los mismos fragmentos de texto, peor leídos, y van a la misma cola.
- **Después la IA**, como otra forma de leer la foto. Se ofrece cuando el lector del móvil no basta.
- La prueba pendiente de [foto-estanteria.md](foto-estanteria.md) (las mismas seis fotos con ML Kit) dirá cuánto falta para que la vía 3 sea útil sola.

## El plugin de Tauri que hay que escribir

**No hay plugin de Tauri para leer texto con la cámara** (ML Kit Text Recognition). Hace falta para dos cosas de la Fase 10:
1. **Leer el ISBN impreso** cuando el código de barras es una etiqueta de tienda (ROADMAP, 05/10/2026).
2. **La foto de balda sin salir del dispositivo** (D19, vía 3).

Se escribe una vez y se publica como **plugin libre de Tauri**, como se pensó para las llaves de acceso: cualquier otra app tiene que poder usarlo.
- **Dónde:** el ROADMAP decía «en su propio repositorio», y el maintainer prefiere no tener muchos (16/10/2026). Un plugin se puede publicar (crates.io y npm) desde una carpeta del repositorio de siempre, `plugins/tauri-plugin-…/`. Mi recomendación es esa, y sacarlo a su propio repositorio solo si llega gente de fuera a trabajar en él. **Decides tú.**
- **Licencia:** MIT o Apache 2.0, como `spec/`, no AGPL. Con AGPL, casi ninguna app lo usaría (D11).
- ML Kit lee el texto en el propio móvil, sin red, como hoy los códigos de barras.
- En el escritorio no hay ML Kit. Si hace falta allí: `ocrs` (Rust) o Tesseract. **Por decidir al empezar:** de primeras, puede ser solo para Android.
