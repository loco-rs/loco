<div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Bienvenido a Loco</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco es Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

Español · [English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Português (Brasil)](./README-pt_BR.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Vietnamese](./README.vi.md) · [العربية](./README.ar.md)

## ¿Qué es Loco?

`Loco` está fuertemente inspirado en Rails. Si conoces Rails y Rust, te sentirás como en casa. Si solo conoces Rails y eres nuevo en Rust, encontrarás Loco refrescante. No asumimos que conozcas Rails.

Para una explicación más profunda de cómo funciona Loco, incluyendo guías detalladas, ejemplos y referencias de la API, consulta nuestro [sitio de documentación](https://loco.rs).

## Características de Loco

* `Convención sobre configuración:` Al igual que Ruby on Rails, Loco enfatiza la simplicidad y la productividad al reducir la necesidad de código repetitivo. Utiliza valores predeterminados sensatos, permitiendo a los desarrolladores centrarse en la lógica de negocio en lugar de perder tiempo en la configuración.

* `Desarrollo rápido:` Loco está diseñado para una alta productividad del desarrollador, reduciendo el código repetitivo y proporcionando APIs intuitivas, permitiendo iterar rápidamente y construir prototipos con un esfuerzo mínimo.

* `Integración ORM:` Modela tu negocio con entidades robustas, eliminando la necesidad de escribir SQL. Define relaciones, validaciones y lógica personalizada directamente en tus entidades para una mayor mantenibilidad y escalabilidad.

* `Controladores:` Maneja parámetros de solicitudes web, cuerpo, validación y renderiza una respuesta consciente del contenido. Usamos Axum para el mejor rendimiento, simplicidad y extensibilidad. Los controladores también permiten construir middlewares fácilmente, que pueden usarse para agregar lógica como autenticación, registro o manejo de errores antes de pasar las solicitudes a las acciones principales del controlador.

* `Vistas:` Loco puede integrarse con motores de plantillas para generar contenido HTML dinámico a partir de plantillas.

* `Trabajos en segundo plano:` Realiza trabajos intensivos en computación o I/O en segundo plano con una cola respaldada por Redis o con hilos. Implementar un worker es tan simple como implementar una función perform para el trait Worker.

* `Planificador:` Simplifica el tradicional y a menudo engorroso sistema crontab, facilitando y haciendo más elegante la programación de tareas o scripts de shell.

* `Mailers:` Un mailer enviará correos electrónicos en segundo plano usando la infraestructura de background worker de Loco. Todo será transparente para ti.

* `Almacenamiento:` En Loco Storage, facilitamos el trabajo con archivos a través de múltiples operaciones. El almacenamiento puede ser en memoria, en disco o usar servicios en la nube como AWS S3, GCP y Azure.

* `Caché:` Loco proporciona una capa de caché para mejorar el rendimiento de la aplicación almacenando datos de acceso frecuente.

Para ver más características de Loco, consulta nuestro [sitio de documentación](https://loco.rs/docs/getting-started/tour/).

## Primeros pasos
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Solo si necesitas base de datos
```
<!-- </snip> -->

Ahora puedes crear tu nueva app (elige "`SaaS` app").

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ ¿Nombre de la app? · miapp
✔ ❯ ¿Qué te gustaría construir? · App SaaS con renderizado del lado del cliente
✔ ❯ Selecciona un proveedor de BD · Sqlite
✔ ❯ Selecciona el tipo de worker en segundo plano · Async (tareas async in-process con tokio)

🚂 App Loco generada exitosamente en:
miapp/

- assets: Has seleccionado `clientside` para la configuración de tu servidor de assets.

Siguiente paso, construye tu frontend:
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

Ahora entra en tu `miapp` y arranca tu app:
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
```sh
$ cargo loco start

                      ▄     ▀
                                ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
██████  █████   ███ █████   ███ █████   ███ ▀█
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
██████  █████   ███ █████       █████   ███ ████▄
██████  █████   ███ █████   ▄▄▄ █████   ███ █████
██████  █████   ███  ████   ███ █████   ███ ████▀
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
                https://loco.rs

listening on port 5150
```
<!-- </snip> -->

## Proyectos impulsados por Loco

* [SpectralOps](https://spectralops.io) - varios servicios impulsados por el framework Loco

* [Nativish](https://nativi.sh) - backend de la app impulsado por el framework Loco

## Contribuidores ✨

Gracias a estas personas maravillosas:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
