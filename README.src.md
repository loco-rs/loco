<!--@nrg.languages=en,es,fr,ja,ko,ru,pt_BR,zh_CN-->
<!--@nrg.defaultLanguage=en-->
<!--@nrg.fileNamePattern.pt_BR=README-pt_BR.md-->
<!--@nrg.fileNamePattern.zh_CN=README-zh_CN.md-->
 <div align="center"><!--en-->
<!--en-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--en-->
<!--en-->
   <h1>Welcome to Loco</h1><!--en-->
<!--en-->
   <h3><!--en-->
   <!-- <snip id="description" inject_from="yaml"> --><!--en-->
🚂 Loco is Rust on Rails.<!--en-->
<!--</snip> --><!--en-->
   </h3><!--en-->
<!--en-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--en-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--en-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--en-->
<!--en-->
 </div><!--en-->
<!--en-->
<!--en-->
English · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md)<!--en-->
<!--en-->
<!--en-->
## What's Loco?<!--en-->
`Loco` is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.<!--en-->
<!--en-->
For a deeper dive into how Loco works, including detailed guides, examples, and API references, check out our [documentation website](https://loco.rs).<!--en-->
<!--en-->
<!--en-->
## Features of Loco:<!--en-->
<!--en-->
* `Convention Over Configuration:` Similar to Ruby on Rails, Loco emphasizes simplicity and productivity by reducing the need for boilerplate code. It uses sensible defaults, allowing developers to focus on writing business logic rather than spending time on configuration.<!--en-->
<!--en-->
* `Rapid Development:` Aim for high developer productivity, Loco’s design focuses on reducing boilerplate code and providing intuitive APIs, allowing developers to iterate quickly and build prototypes with minimal effort.<!--en-->
<!--en-->
* `ORM Integration:` Model your business with robust entities, eliminating the need to write SQL. Define relationships, validation, and custom logic directly on your entities for enhanced maintainability and scalability.<!--en-->
<!--en-->
* `Controllers`: Handle web requests parameters, body, validation, and render a response that is content-aware. We use Axum for the best performance, simplicity, and extensibility. Controllers also allow you to easily build middlewares, which can be used to add logic such as authentication, logging, or error handling before passing requests to the main controller actions.<!--en-->
<!--en-->
* `Views:` Loco can integrate with templating engines to generate dynamic HTML content from templates.<!--en-->
<!--en-->
* `Background Jobs:` Perform compute or I/O intensive jobs in the background with a Redis backed queue, or with threads. Implementing a worker is as simple as implementing a perform function for the Worker trait.<!--en-->
<!--en-->
* `Scheduler:` Simplifies the traditional, often cumbersome crontab system, making it easier and more elegant to schedule tasks or shell scripts.<!--en-->
<!--en-->
* `Mailers:` A mailer will deliver emails in the background using the existing loco background worker infrastructure. It will all be seamless for you.<!--en-->
<!--en-->
* `Storage:` In Loco Storage, we facilitate working with files through multiple operations. Storage can be in-memory, on disk, or use cloud services such as AWS S3, GCP, and Azure.<!--en-->
<!--en-->
* `Cache:` Loco provides an cache layer to improve application performance by storing frequently accessed data.<!--en-->
<!--en-->
So see more Loco features, check out our [documentation website](https://loco.rs/docs/getting-started/tour/).<!--en-->
<!--en-->
<!--en-->
<!--en-->
## Getting Started<!--en-->
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> --><!--en-->
```sh<!--en-->
cargo install loco<!--en-->
cargo install sea-orm-cli # Only when DB is needed<!--en-->
```<!--en-->
<!-- </snip> --><!--en-->
<!--en-->
Now you can create your new app (choose "`SaaS` app").<!--en-->
<!--en-->
<!--en-->
<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> --><!--en-->
```sh<!--en-->
❯ loco new<!--en-->
✔ ❯ App name? · myapp<!--en-->
✔ ❯ What would you like to build? · Saas App with client side rendering<!--en-->
✔ ❯ Select a DB Provider · Sqlite<!--en-->
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)<!--en-->
<!--en-->
🚂 Loco app generated successfully in:<!--en-->
myapp/<!--en-->
<!--en-->
- assets: You've selected `clientside` for your asset serving configuration.<!--en-->
<!--en-->
Next step, build your frontend:<!--en-->
  $ cd frontend/<!--en-->
  $ npm install && npm run build<!--en-->
```<!--en-->
<!-- </snip> --><!--en-->
<!--en-->
 Now `cd` into your `myapp` and start your app:<!--en-->
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> --><!--en-->
```sh<!--en-->
$ cargo loco start<!--en-->
<!--en-->
                      ▄     ▀<!--en-->
                                ▀  ▄<!--en-->
                  ▄       ▀     ▄  ▄ ▄▀<!--en-->
                                    ▄ ▀▄▄<!--en-->
                        ▄     ▀    ▀  ▀▄▀█▄<!--en-->
                                          ▀█▄<!--en-->
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█<!--en-->
██████  █████   ███ █████   ███ █████   ███ ▀█<!--en-->
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄<!--en-->
██████  █████   ███ █████       █████   ███ ████▄<!--en-->
██████  █████   ███ █████   ▄▄▄ █████   ███ █████<!--en-->
██████  █████   ███  ████   ███ █████   ███ ████▀<!--en-->
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀<!--en-->
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀<!--en-->
                https://loco.rs<!--en-->
<!--en-->
listening on port 5150<!--en-->
```<!--en-->
<!-- </snip> --><!--en-->
<!--en-->
## Powered by Loco<!--en-->
+ [SpectralOps](https://spectralops.io) - various services powered by Loco<!--en-->
  framework<!--en-->
+ [Nativish](https://nativi.sh) - app backend powered by Loco framework<!--en-->
<!--en-->
## Contributors ✨<!--en-->
Thanks goes to these wonderful people:<!--en-->
<!--en-->
<a href="https://github.com/loco-rs/loco/graphs/contributors"><!--en-->
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" /><!--en-->
</a><!--en-->
<div align="center"><!--es-->
<!--es-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--es-->
<!--es-->
   <h1>Bienvenido a Loco</h1><!--es-->
<!--es-->
   <h3><!--es-->
   <!-- <snip id="description" inject_from="yaml"> --><!--es-->
🚂 Loco es Rust on Rails.<!--es-->
<!--</snip> --><!--es-->
   </h3><!--es-->
<!--es-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--es-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--es-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--es-->
<!--es-->
 </div><!--es-->
<!--es-->
Español · [English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Português (Brasil)](./README-pt_BR.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · Español<!--es-->
<!--es-->
## ¿Qué es Loco?<!--es-->
<!--es-->
`Loco` está fuertemente inspirado en Rails. Si conoces Rails y Rust, te sentirás como en casa. Si solo conoces Rails y eres nuevo en Rust, encontrarás Loco refrescante. No asumimos que conozcas Rails.<!--es-->
<!--es-->
Para una explicación más profunda de cómo funciona Loco, incluyendo guías detalladas, ejemplos y referencias de la API, consulta nuestro [sitio de documentación](https://loco.rs).<!--es-->
<!--es-->
## Características de Loco<!--es-->
<!--es-->
* `Convención sobre configuración:` Al igual que Ruby on Rails, Loco enfatiza la simplicidad y la productividad al reducir la necesidad de código repetitivo. Utiliza valores predeterminados sensatos, permitiendo a los desarrolladores centrarse en la lógica de negocio en lugar de perder tiempo en la configuración.<!--es-->
<!--es-->
* `Desarrollo rápido:` Loco está diseñado para una alta productividad del desarrollador, reduciendo el código repetitivo y proporcionando APIs intuitivas, permitiendo iterar rápidamente y construir prototipos con un esfuerzo mínimo.<!--es-->
<!--es-->
* `Integración ORM:` Modela tu negocio con entidades robustas, eliminando la necesidad de escribir SQL. Define relaciones, validaciones y lógica personalizada directamente en tus entidades para una mayor mantenibilidad y escalabilidad.<!--es-->
<!--es-->
* `Controladores:` Maneja parámetros de solicitudes web, cuerpo, validación y renderiza una respuesta consciente del contenido. Usamos Axum para el mejor rendimiento, simplicidad y extensibilidad. Los controladores también permiten construir middlewares fácilmente, que pueden usarse para agregar lógica como autenticación, registro o manejo de errores antes de pasar las solicitudes a las acciones principales del controlador.<!--es-->
<!--es-->
* `Vistas:` Loco puede integrarse con motores de plantillas para generar contenido HTML dinámico a partir de plantillas.<!--es-->
<!--es-->
* `Trabajos en segundo plano:` Realiza trabajos intensivos en computación o I/O en segundo plano con una cola respaldada por Redis o con hilos. Implementar un worker es tan simple como implementar una función perform para el trait Worker.<!--es-->
<!--es-->
* `Planificador:` Simplifica el tradicional y a menudo engorroso sistema crontab, facilitando y haciendo más elegante la programación de tareas o scripts de shell.<!--es-->
<!--es-->
* `Mailers:` Un mailer enviará correos electrónicos en segundo plano usando la infraestructura de background worker de Loco. Todo será transparente para ti.<!--es-->
<!--es-->
* `Almacenamiento:` En Loco Storage, facilitamos el trabajo con archivos a través de múltiples operaciones. El almacenamiento puede ser en memoria, en disco o usar servicios en la nube como AWS S3, GCP y Azure.<!--es-->
<!--es-->
* `Caché:` Loco proporciona una capa de caché para mejorar el rendimiento de la aplicación almacenando datos de acceso frecuente.<!--es-->
<!--es-->
Para ver más características de Loco, consulta nuestro [sitio de documentación](https://loco.rs/docs/getting-started/tour/).<!--es-->
<!--es-->
## Primeros pasos<!--es-->
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> --><!--es-->
```sh<!--es-->
cargo install loco<!--es-->
cargo install sea-orm-cli # Solo si necesitas base de datos<!--es-->
```<!--es-->
<!-- </snip> --><!--es-->
<!--es-->
Ahora puedes crear tu nueva app (elige "`SaaS` app").<!--es-->
<!--es-->
<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> --><!--es-->
```sh<!--es-->
❯ loco new<!--es-->
✔ ❯ ¿Nombre de la app? · miapp<!--es-->
✔ ❯ ¿Qué te gustaría construir? · App SaaS con renderizado del lado del cliente<!--es-->
✔ ❯ Selecciona un proveedor de BD · Sqlite<!--es-->
✔ ❯ Selecciona el tipo de worker en segundo plano · Async (tareas async in-process con tokio)<!--es-->
<!--es-->
🚂 App Loco generada exitosamente en:<!--es-->
miapp/<!--es-->
<!--es-->
- assets: Has seleccionado `clientside` para la configuración de tu servidor de assets.<!--es-->
<!--es-->
Siguiente paso, construye tu frontend:<!--es-->
  $ cd frontend/<!--es-->
  $ npm install && npm run build<!--es-->
```<!--es-->
<!-- </snip> --><!--es-->
<!--es-->
Ahora entra en tu `miapp` y arranca tu app:<!--es-->
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> --><!--es-->
```sh<!--es-->
$ cargo loco start<!--es-->
<!--es-->
                      ▄     ▀<!--es-->
                                ▀  ▄<!--es-->
                  ▄       ▀     ▄  ▄ ▄▀<!--es-->
                                    ▄ ▀▄▄<!--es-->
                        ▄     ▀    ▀  ▀▄▀█▄<!--es-->
                                          ▀█▄<!--es-->
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█<!--es-->
██████  █████   ███ █████   ███ █████   ███ ▀█<!--es-->
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄<!--es-->
██████  █████   ███ █████       █████   ███ ████▄<!--es-->
██████  █████   ███ █████   ▄▄▄ █████   ███ █████<!--es-->
██████  █████   ███  ████   ███ █████   ███ ████▀<!--es-->
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀<!--es-->
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀<!--es-->
                https://loco.rs<!--es-->
<!--es-->
listening on port 5150<!--es-->
```<!--es-->
<!-- </snip> --><!--es-->
<!--es-->
## Proyectos impulsados por Loco<!--es-->
<!--es-->
* [SpectralOps](https://spectralops.io) - varios servicios impulsados por el framework Loco<!--es-->
<!--es-->
* [Nativish](https://nativi.sh) - backend de la app impulsado por el framework Loco<!--es-->
<!--es-->
## Contribuidores ✨<!--es-->
<!--es-->
Gracias a estas personas maravillosas:<!--es-->
<!--es-->
<a href="https://github.com/loco-rs/loco/graphs/contributors"><!--es-->
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" /><!--es-->
</a><!--es-->
 <div align="center"><!--fr-->
<!--fr-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--fr-->
<!--fr-->
   <h1>Loco vous souhaite la bienvenue</h1><!--fr-->
<!--fr-->
   <h3><!--fr-->
🚂 Loco c'est Rust on Rails.<!--fr-->
   </h3><!--fr-->
<!--fr-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--fr-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--fr-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--fr-->
<!--fr-->
 </div><!--fr-->
<!--fr-->
[English](./README.md) · [中文](./README-zh_CN.md) · Français · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md)<!--fr-->
<!--fr-->
## À propos de Loco<!--fr-->
`Loco` est fortement inspiré de Rails. Si vous connaissez Rails et Rust, vous vous sentirez chez vous. Si vous ne connaissez que Rails et que vous êtes nouveau sur Rust, vous trouverez Loco rafraîchissant. Nous ne supposons pas que vous connaissez Rails.<!--fr-->
Pour un aperçu plus approfondie du fonctionnement de Loco, y compris des guides détaillés, des exemples et des références API, consultez notre [site Web de documentation](https://loco.rs).<!--fr-->
<!--fr-->
## Caractéristiques de Loco:<!--fr-->
<!--fr-->
* `Convention plutôt que configuration`: Semblable à Ruby on Rails, Loco met l'accent sur la simplicité et la productivité en réduisant le besoin de code passe-partout. Il utilise des valeurs par défaut raisonnables, permettant aux développeurs de se concentrer sur l'écriture de la logique métier plutôt que de consacrer du temps à la configuration.<!--fr-->
<!--fr-->
* `Développement rapide`: Visant une productivité élevée des développeurs, la conception de Loco se concentre sur la réduction du code passe-partout et la fourniture d'API intuitives, permettant aux développeurs d'intégrer rapidement et de créer des prototypes avec un minimum d'effort.<!--fr-->
<!--fr-->
* `Intégration ORM`: Modélisez avec des entités robustes, éliminant le besoin d'écrire du SQL. Définissez les relations, la validation et la logique sur mesure directement sur vos entités pour une maintenabilité et une évolutivité améliorées.<!--fr-->
<!--fr-->
* `Contrôleurs`: Gérez les paramètres et le contenu des requêtes Web, la validation des requêtes et affichez une réponse tenant compte du contenu. Nous utilisons Axum pour une meilleure performance, simplicité et extensibilité. Les contrôleurs vous permettent également de créer facilement des middlewares, qui peuvent être utilisés pour ajouter une logique telle que l'authentification, la journalisation (logging) ou la gestion des erreurs avant de transmettre les requêtes aux actions du contrôleur principal.<!--fr-->
<!--fr-->
* `Vues`: Loco peut s'intégrer aux moteurs de _templates_ pour générer du contenu HTML dynamique à partir de modèles template.<!--fr-->
<!--fr-->
* `Tâches en arrière-plan`: Effectuer des calculs informatiques ou d'I/O (Entrée/Sortie) intensives en arrière-plan avec une file d'attente sauvegardée Redis ou avec des threads. Implémenter un travailleur (worker) est aussi simple que d'implémenter une fonction d'exécution pour le trait Worker.<!--fr-->
<!--fr-->
* `Scheduler`: Simplifie le système crontab traditionnel, souvent encombrant, en rendant plus facile et plus élégante la planification de tâches ou de scripts shell.<!--fr-->
<!--fr-->
* `Mailers`: Un logiciel de messagerie enverra des e-mails en arrière-plan en utilisant l'infrastructure de travail d'arrière-plan de Loco existante. Tout se passera sans problème pour vous.<!--fr-->
<!--fr-->
* `Stockage`: Loco Storage facilite le travail avec des fichiers via plusieurs opérations. Le stockage peut être en mémoire, sur disque ou utiliser des services cloud tels qu'AWS S3, GCP et Azure.<!--fr-->
<!--fr-->
* `Cache :` Loco fournit une strate cache pour améliorer les performances des applications en stockant les données fréquemment consultées.<!--fr-->
<!--fr-->
Pour en savoir plus sur les fonctionnalités de Loco, consultez notre [site Web de documentation](https://loco.rs/docs/getting-started/tour/).<!--fr-->
<!--fr-->
<!--fr-->
## Commencez rapidement<!--fr-->
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> --><!--fr-->
```sh<!--fr-->
cargo install loco<!--fr-->
cargo install sea-orm-cli # Only when DB is needed<!--fr-->
```<!--fr-->
<!-- </snip> --><!--fr-->
<!--fr-->
Vous pouvez maintenant créer votre nouvelle application (choisissez "`SaaS` app").<!--fr-->
<!--fr-->
<!--fr-->
<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> --><!--fr-->
```sh<!--fr-->
❯ loco new<!--fr-->
✔ ❯ App name? · myapp<!--fr-->
✔ ❯ What would you like to build? · Saas App with client side rendering<!--fr-->
✔ ❯ Select a DB Provider · Sqlite<!--fr-->
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)<!--fr-->
<!--fr-->
🚂 Loco app generated successfully in:<!--fr-->
myapp/<!--fr-->
<!--fr-->
- assets: You've selected `clientside` for your asset serving configuration.<!--fr-->
<!--fr-->
Next step, build your frontend:<!--fr-->
  $ cd frontend/<!--fr-->
  $ npm install && npm run build<!--fr-->
```<!--fr-->
<!-- </snip> --><!--fr-->
<!--fr-->
Maintenant, faite `cd` dans votre `myapp` et démarrez votre application:<!--fr-->
<!--fr-->
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> --><!--fr-->
```sh<!--fr-->
$ cargo loco start<!--fr-->
<!--fr-->
                      ▄     ▀<!--fr-->
                                ▀  ▄<!--fr-->
                  ▄       ▀     ▄  ▄ ▄▀<!--fr-->
                                    ▄ ▀▄▄<!--fr-->
                        ▄     ▀    ▀  ▀▄▀█▄<!--fr-->
                                          ▀█▄<!--fr-->
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█<!--fr-->
██████  █████   ███ █████   ███ █████   ███ ▀█<!--fr-->
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄<!--fr-->
██████  █████   ███ █████       █████   ███ ████▄<!--fr-->
██████  █████   ███ █████   ▄▄▄ █████   ███ █████<!--fr-->
██████  █████   ███  ████   ███ █████   ███ ████▀<!--fr-->
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀<!--fr-->
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀<!--fr-->
                https://loco.rs<!--fr-->
<!--fr-->
listening on port 5150<!--fr-->
```<!--fr-->
<!-- </snip> --><!--fr-->
<!--fr-->
## Servi par Loco<!--fr-->
+ [SpectralOps](https://spectralops.io) - divers services servi par le framework Loco<!--fr-->
+ [Nativish](https://nativi.sh) - app backend servi par le framework Loco<!--fr-->
<!--fr-->
## Contributeurs ✨<!--fr-->
Merci à ces personnes formidables :<!--fr-->
<!--fr-->
<a href="https://github.com/loco-rs/loco/graphs/contributors"><!--fr-->
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" /><!--fr-->
</a><!--fr-->
<!--fr-->
<div align="center"><!--ja-->
<!--ja-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--ja-->
<!--ja-->
   <h1>Locoへようこそ</h1><!--ja-->
<!--ja-->
   <h3><!--ja-->
🚂 LocoはRust on Railsです。<!--ja-->
   </h3><!--ja-->
<!--ja-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--ja-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--ja-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--ja-->
<!--ja-->
 </div><!--ja-->
<!--ja-->
English · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ 日本語 · [한국어](./README.ko.md) · [Русский](./README.ru.md)<!--ja-->
<!--ja-->
## Locoとは？<!--ja-->
`Loco`はRailsに強くインスパイアされています。RailsとRustの両方を知っているなら、すぐに馴染むでしょう。Railsしか知らなく、Rustに新しい方でも、Locoは新鮮に感じるでしょう。Railsを知っているとは仮定していません。<!--ja-->
<!--ja-->
Locoの動作についての詳細なガイド、例、APIリファレンスは、[ドキュメント](https://loco.rs)をチェックしてください。<!--ja-->
<!--ja-->
## Locoの特徴：<!--ja-->
<!--ja-->
* `設定より規約:` Ruby on Railsに似て、Locoはボイラープレートコードを減らすことでシンプルさと生産性を発揮します。合理的なデフォルトを使用し、開発者が設定に時間を費やすのではなく、ビジネスロジックの記述に集中できるようにします。<!--ja-->
<!--ja-->
* `迅速な開発:` 高い開発者生産性を目指し、Locoの設計はボイラープレートコードを減らし、直感的なAPIを提供することに焦点を当てています。これにより、開発者は迅速に反復し、最小限の努力でプロトタイプを構築できます。<!--ja-->
<!--ja-->
* `ORM統合:` ビジネスモデルを堅牢なエンティティで表現し、SQLを書く必要をなくします。エンティティに直接関係、検証、およびカスタムロジックを定義でき、メンテナンス性とスケーラビリティが向上します。<!--ja-->
<!--ja-->
* `コントローラー:` ウェブリクエストのパラメータ、ボディ、検証を処理し、コンテンツに応じたレスポンスをレンダリングします。最高のパフォーマンス、シンプルさ、拡張性のためにAxumを使用しています。コントローラーは、認証、ロギング、エラーハンドリングなどのロジックを追加するためのミドルウェアを簡単に構築できます。<!--ja-->
<!--ja-->
* `ビュー:` Locoはテンプレートエンジンと統合し、テンプレートから動的なHTMLコンテンツを生成できます。<!--ja-->
<!--ja-->
* `バックグラウンドジョブ:` Redisバックエンドキューやスレッドを使用して、計算またはI/O集約型のジョブをバックグラウンドで実行します。ワーカーを実装するのは、Workerトレイトのperform関数を実装するだけです。<!--ja-->
<!--ja-->
* `スケジューラー:` 従来の、しばしば面倒なcrontabシステムを簡素化し、タスクやシェルスクリプトをスケジュールするのをより簡単かつエレガントにします。<!--ja-->
<!--ja-->
* `メール送信:` メール送信者は、既存のLocoバックグラウンドワーカーインフラストラクチャを使用して、バックグラウンドでメールを配信します。すべてがシームレスに行われます。<!--ja-->
<!--ja-->
* `ストレージ:` Locoのストレージでは、ファイル操作を簡素化します。ストレージはメモリ内、ディスク上、またはAWS S3、GCP、Azureなどのクラウドサービスを使用できます。<!--ja-->
<!--ja-->
* `キャッシュ:` Locoは、頻繁にアクセスされるデータを保存することでアプリケーションのパフォーマンスを向上させるためのキャッシュレイヤーを提供します。<!--ja-->
<!--ja-->
Locoの詳細な機能については、[ドキュメントウェブサイト](https://loco.rs/docs/getting-started/tour/)を確認してください。<!--ja-->
<!--ja-->
## 始め方<!--ja-->
```sh<!--ja-->
cargo install loco<!--ja-->
cargo install sea-orm-cli # データベースが必要な場合のみ<!--ja-->
```<!--ja-->
<!--ja-->
以下で新しいアプリを作成できます（「`SaaS`アプリ」を選択）。<!--ja-->
<!--ja-->
```sh<!--ja-->
❯ loco new<!--ja-->
✔ ❯ App name? · myapp<!--ja-->
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)<!--ja-->
✔ ❯ Select a DB Provider · Sqlite<!--ja-->
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)<!--ja-->
✔ ❯ Select an asset serving configuration · Client (configures assets for frontend serving)<!--ja-->
<!--ja-->
🚂 Loco app generated successfully in:<!--ja-->
myapp/<!--ja-->
```<!--ja-->
<!--ja-->
次に`myapp`に移動し、アプリを起動します：<!--ja-->
```sh<!--ja-->
$ cargo loco start<!--ja-->
<!--ja-->
                      ▄     ▀<!--ja-->
                                ▀  ▄<!--ja-->
                  ▄       ▀     ▄  ▄ ▄▀<!--ja-->
                                    ▄ ▀▄▄<!--ja-->
                        ▄     ▀    ▀  ▀▄▀█▄<!--ja-->
                                          ▀█▄<!--ja-->
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█<!--ja-->
██████  █████   ███ █████   ███ █████   ███ ▀█<!--ja-->
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄<!--ja-->
██████  █████   ███ █████       █████   ███ ████▄<!--ja-->
██████  █████   ███ █████   ▄▄▄ █████   ███ █████<!--ja-->
██████  █████   ███  ████   ███ █████   ███ ████▀<!--ja-->
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀<!--ja-->
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀<!--ja-->
                https://loco.rs<!--ja-->
<!--ja-->
listening on port 5150<!--ja-->
```<!--ja-->
<!--ja-->
## Locoによって開発されています<!--ja-->
+ [SpectralOps](https://spectralops.io) - Locoフレームワークによる各種サービス<!--ja-->
+ [Nativish](https://nativi.sh) - Locoフレームワークによるアプリバックエンド<!--ja-->
<!--ja-->
## 貢献者 ✨<!--ja-->
これらの素晴らしい人々に感謝します：<!--ja-->
<!--ja-->
<a href="https://github.com/loco-rs/loco/graphs/contributors"><!--ja-->
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" /><!--ja-->
</a><!--ja-->
 <div align="center"><!--ko-->
<!--ko-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--ko-->
<!--ko-->
   <h1>Loco에 오신 것을 환영합니다</h1><!--ko-->
<!--ko-->
   <h3><!--ko-->
   🚂 Loco는 Rust on Rails입니다.<!--ko-->
   </h3><!--ko-->
<!--ko-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--ko-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--ko-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--ko-->
<!--ko-->
 </div><!--ko-->
<!--ko-->
[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · 한국어 · [Русский](./README.ru.md) · [Español](./README.es.md)<!--ko-->
<!--ko-->
<!--ko-->
## Loco란?<!--ko-->
`Loco`는 Rails에서 강한 영감을 받았습니다. Rails와 Rust를 모두 알고 계신다면 친숙하게 느껴지실 것이며, Rails만 알고 Rust를 처음 접하시는 분들에게도 Loco는 새롭게 다가올 것입니다. 참고로, Rails에 대한 사전 지식은 필수가 아닙니다.<!--ko-->
<!--ko-->
Loco의 작동 방식에 대해 더 자세히 알아보려면 가이드, 예제, API 참조를 포함한 [문서 웹사이트](https://loco.rs)를 확인해보세요.<!--ko-->
<!--ko-->
## Loco의 주요 기능:<!--ko-->
<!--ko-->
* `설정보다 관습`: Ruby on Rails와 유사하게, Loco는 상용구 코드의 필요성을 줄임으로써 단순성과 생산성을 강조합니다. 합리적인 기본값을 사용하여 개발자가 설정보다는 비즈니스 로직 작성에 집중할 수 있게 합니다.<!--ko-->
<!--ko-->
* `빠른 개발`: 높은 개발자 생산성을 목표로 하며, Loco의 설계는 상용구 코드를 줄이고 직관적인 API를 제공하여 개발자가 최소한의 노력으로 빠르게 반복하고 프로토타입을 구축할 수 있도록 합니다.<!--ko-->
<!--ko-->
* `ORM 통합`: SQL 작성 없이 비즈니스를 강력한 엔티티로 모델링합니다. 관계, 유효성 검사, 사용자 정의 로직을 엔티티에 직접 정의하여 유지보수성과 확장성을 향상시킵니다.<!--ko-->
<!--ko-->
* `컨트롤러`: 웹 요청 매개변수, 본문, 유효성 검사를 처리하고 컨텐츠를 인식하는 응답을 렌더링합니다. 최고의 성능, 단순성, 확장성을 위해 Axum을 사용합니다. 또한 컨트롤러를 통해 인증, 로깅, 오류 처리와 같은 로직을 추가할 수 있는 미들웨어를 쉽게 구축할 수 있습니다.<!--ko-->
<!--ko-->
* `뷰`: Loco는 템플릿에서 동적 HTML 콘텐츠를 생성하기 위해 템플릿 엔진과 통합할 수 있습니다.<!--ko-->
<!--ko-->
* `백그라운드 작업`: Redis 기반 큐 또는 스레드를 사용하여 계산이나 I/O 집약적인 작업을 백그라운드에서 수행합니다. Worker 트레이트에 대한 perform 함수를 구현하는 것만으로도 워커를 구현할 수 있습니다.<!--ko-->
<!--ko-->
* `스케줄러`: 전통적이고 번거로운 crontab 시스템을 단순화하여 작업이나 셸 스크립트를 더 쉽고 우아하게 예약할 수 있습니다.<!--ko-->
<!--ko-->
* `메일러`: 메일러는 기존 loco 백그라운드 워커 인프라를 사용하여 이메일을 백그라운드에서 전달합니다. 모든 과정이 매끄럽게 처리됩니다.<!--ko-->
<!--ko-->
* `스토리지`: Loco 스토리지는 여러 작업을 통해 파일 작업을 용이하게 합니다. 메모리 내, 디스크, AWS S3, GCP, Azure와 같은 클라우드 서비스를 사용할 수 있습니다.<!--ko-->
<!--ko-->
* `캐시`: Loco는 자주 접근하는 데이터를 저장하여 애플리케이션 성능을 향상시키는 캐시 레이어를 제공합니다.<!--ko-->
<!--ko-->
더 많은 Loco 기능을 보려면 [문서 웹사이트](https://loco.rs/docs/getting-started/tour/)를 확인하세요.<!--ko-->
<!--ko-->
<!--ko-->
## 시작하기<!--ko-->
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> --><!--ko-->
```sh<!--ko-->
cargo install loco<!--ko-->
cargo install sea-orm-cli # Only when DB is needed<!--ko-->
```<!--ko-->
<!-- </snip> --><!--ko-->
<!--ko-->
이제 새로운 앱을 만들 수 있습니다 ("`SaaS 앱`" 선택).<!--ko-->
<!--ko-->
<!--ko-->
<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> --><!--ko-->
```sh<!--ko-->
❯ loco new<!--ko-->
✔ ❯ App name? · myapp<!--ko-->
✔ ❯ What would you like to build? · Saas App with client side rendering<!--ko-->
✔ ❯ Select a DB Provider · Sqlite<!--ko-->
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)<!--ko-->
<!--ko-->
🚂 Loco app generated successfully in:<!--ko-->
myapp/<!--ko-->
<!--ko-->
- assets: You've selected `clientside` for your asset serving configuration.<!--ko-->
<!--ko-->
Next step, build your frontend:<!--ko-->
  $ cd frontend/<!--ko-->
  $ npm install && npm run build<!--ko-->
```<!--ko-->
<!-- </snip> --><!--ko-->
<!--ko-->
이제 `myapp` 디렉토리로 이동하여 앱을 시작하세요:<!--ko-->
<!--ko-->
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> --><!--ko-->
```sh<!--ko-->
$ cargo loco start<!--ko-->
<!--ko-->
                      ▄     ▀<!--ko-->
                                ▀  ▄<!--ko-->
                  ▄       ▀     ▄  ▄ ▄▀<!--ko-->
                                    ▄ ▀▄▄<!--ko-->
                        ▄     ▀    ▀  ▀▄▀█▄<!--ko-->
                                          ▀█▄<!--ko-->
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█<!--ko-->
██████  █████   ███ █████   ███ █████   ███ ▀█<!--ko-->
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄<!--ko-->
██████  █████   ███ █████       █████   ███ ████▄<!--ko-->
██████  █████   ███ █████   ▄▄▄ █████   ███ █████<!--ko-->
██████  █████   ███  ████   ███ █████   ███ ████▀<!--ko-->
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀<!--ko-->
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀<!--ko-->
                https://loco.rs<!--ko-->
<!--ko-->
listening on port 5150<!--ko-->
```<!--ko-->
<!-- </snip> --><!--ko-->
<!--ko-->
## Loco 사용 사례<!--ko-->
+ [SpectralOps](https://spectralops.io) - Loco 프레임워크로 구동되는 다양한 서비스<!--ko-->
+ [Nativish](https://nativi.sh) - Loco 프레임워크로 구동되는 앱 백엔드<!--ko-->
<!--ko-->
## 기여자 ✨<!--ko-->
이 멋진 분들께 감사드립니다:<!--ko-->
<!--ko-->
<a href="https://github.com/loco-rs/loco/graphs/contributors"><!--ko-->
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" /><!--ko-->
</a><!--ko-->
 <div align="center"><!--ru-->
<!--ru-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--ru-->
<!--ru-->
   <h1>Добро пожаловать в *Loco*</h1><!--ru-->
<!--ru-->
   <h3><!--ru-->
   <!-- <snip id="description" inject_from="yaml"> --><!--ru-->
🚂 Loco is Rust on Rails.<!--ru-->
<!--</snip> --><!--ru-->
   </h3><!--ru-->
<!--ru-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--ru-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--ru-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--ru-->
<!--ru-->
 </div><!--ru-->
<!--ru-->
[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · Русский · [Español](./README.es.md)<!--ru-->
<!--ru-->
<!--ru-->
## Что такое Loco?<!--ru-->
*Loco* сильно вдохновлён проектом *Ruby on Rails*. Если вы знакомы и с *Rails*, и с *Rust*, вы будете чувствовать себя как дома. Если вы знаете только *Rails*, и не знакомы с *Rust*, *Loco* будет для вас чем-то освежающим.<!--ru-->
<!--ru-->
Если вам интересно узнать внутрение устройство *Loco*, включая детальные гайды, примеры, и устройство API, почитайте нашу [документацию](https://loco.rs).<!--ru-->
<!--ru-->
<!--ru-->
## Фишки Loco:<!--ru-->
<!--ru-->
- **Простота превыше конфигурации**: Подобно *Ruby on Rails*, *Loco* делает упор на простоту и продуктивность, снижая потребность в лишнем коде. *Loco* использует оптимальные настройки по-умолчанию, давая разработчикам возможность сфокусироваться на написании бизнес логики, а не конфигурации.<!--ru-->
- **Быстрая разработка**: Ставя акцент на высокой производительности разработчика, Дизайн *Loco* фокусируется на сокращении ненужного кода и предоставления интуитивного API. Это позволяет быстро создавать прототипы без лишних усилий.<!--ru-->
- **ORM интеграция**: Стройте свой бизнес с крепкими составляющими, убирая необходимость писать SQL. Определяйте взаимосвязи, проверку, и кастомную логику прямо в составляющих, упрощая поддержку и рост кодовой базы.<!--ru-->
- **Контролеры**: Обрабатывайте параметры и данные web-запросов, проверяйте их содержимое, отображайте ответ с учетом запроса. Мы используем *Axum* для достижения наилучшей производительности, простоты, и возможности расширения. Также, контролеры облегчают внедрение middleware. Это может быть использовано для добавления всевозможной логики: аутентификации, логгинга, или обработки ошибок перед отправкой на сервер.<!--ru-->
- **Виды**: *Loco* может интегрироваться с template-движками для генерации динамического HTML из шаблонов.<!--ru-->
- **Фоновые задачи**: Исполняйте I/O и другие тяжелые операции в фоновом режиме с помощью *Redis*, или потоков. Для написания функционала фоновой задачи нужно всего лишь написать функцию `perform` из `trait Worker`.<!--ru-->
- **Планировщик**: Облегчает традиционную, часто громоздкую систему, упрощая планировку задач и исполнение shell-скриптов.<!--ru-->
- **Отправка электронной почты**: Отправка электронной почты в фоновом режиме, без необходимости создавать новую фоновую задачу.<!--ru-->
- **Хранилище**: Мы способствуем работе с файлами несколькими путями: хранение в памяти, на диске, или использование облачных сервисов как *AWS*, *S3*, *GCP*, и *Azure*.<!--ru-->
- **Кэширование**: *Loco* кэширует частые запросы для улучшения производительности приложения.<!--ru-->
<!--ru-->
У *Loco* есть ещё множество фишек, котрые вы можете посмотреть на [сайте документации](https://loco.rs/docs/getting-started/tour/).<!--ru-->
<!--ru-->
<!--ru-->
## Установка<!--ru-->
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> --><!--ru-->
```sh<!--ru-->
cargo install loco<!--ru-->
cargo install sea-orm-cli # Only when DB is needed<!--ru-->
```<!--ru-->
<!-- </snip> --><!--ru-->
<!--ru-->
Теперь вы можете создать свое новое приложение (выберете "`SaaS` app").<!--ru-->
<!--ru-->
<!--ru-->
<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> --><!--ru-->
```sh<!--ru-->
❯ loco new<!--ru-->
✔ ❯ App name? · myapp<!--ru-->
✔ ❯ What would you like to build? · Saas App with client side rendering<!--ru-->
✔ ❯ Select a DB Provider · Sqlite<!--ru-->
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)<!--ru-->
<!--ru-->
🚂 Loco app generated successfully in:<!--ru-->
myapp/<!--ru-->
<!--ru-->
- assets: You've selected `clientside` for your asset serving configuration.<!--ru-->
<!--ru-->
Next step, build your frontend:<!--ru-->
  $ cd frontend/<!--ru-->
  $ npm install && npm run build<!--ru-->
```<!--ru-->
<!-- </snip> --><!--ru-->
<!--ru-->
Теперь выполните `cd` в папку `myapp` и запускайте приложение:<!--ru-->
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> --><!--ru-->
```sh<!--ru-->
$ cargo loco start<!--ru-->
<!--ru-->
                      ▄     ▀<!--ru-->
                                ▀  ▄<!--ru-->
                  ▄       ▀     ▄  ▄ ▄▀<!--ru-->
                                    ▄ ▀▄▄<!--ru-->
                        ▄     ▀    ▀  ▀▄▀█▄<!--ru-->
                                          ▀█▄<!--ru-->
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█<!--ru-->
██████  █████   ███ █████   ███ █████   ███ ▀█<!--ru-->
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄<!--ru-->
██████  █████   ███ █████       █████   ███ ████▄<!--ru-->
██████  █████   ███ █████   ▄▄▄ █████   ███ █████<!--ru-->
██████  █████   ███  ████   ███ █████   ███ ████▀<!--ru-->
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀<!--ru-->
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀<!--ru-->
                https://loco.rs<!--ru-->
<!--ru-->
listening on port 5150<!--ru-->
```<!--ru-->
<!-- </snip> --><!--ru-->
<!--ru-->
## Проекты, использующие *Loco*<!--ru-->
+ [SpectralOps](https://spectralops.io) - различные сервисы, использующие *Loco*<!--ru-->
  framework<!--ru-->
+ [Nativish](https://nativi.sh) - backend приложения, использующий *Loco*<!--ru-->
<!--ru-->
## Контрибьютеры ✨<!--ru-->
Спасибо всем этим прекрасным людям:<!--ru-->
<!--ru-->
<a href="https://github.com/loco-rs/loco/graphs/contributors"><!--ru-->
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" /><!--ru-->
</a><!--ru-->
 <div align="center"><!--pt_BR-->
<!--pt_BR-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--pt_BR-->
<!--pt_BR-->
   <h1>Bem-vindo ao Loco</h1><!--pt_BR-->
<!--pt_BR-->
   <h3><!--pt_BR-->
   <!-- <snip id="description" inject_from="yaml"> --><!--pt_BR-->
🚂 Loco is Rust on Rails.<!--pt_BR-->
<!--</snip> --><!--pt_BR-->
   </h3><!--pt_BR-->
<!--pt_BR-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--pt_BR-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--pt_BR-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--pt_BR-->
<!--pt_BR-->
 </div><!--pt_BR-->
<!--pt_BR-->
[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · Portuguese (Brazil) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md)<!--pt_BR-->
<!--pt_BR-->
<!--pt_BR-->
## O que é o Loco?<!--pt_BR-->
`Loco` é fortemente inspirado no Rails. Se você conhece Rails e Rust, se sentirá em casa. Se você só conhece Rails e é novo em Rust, achará o Loco refrescante. Não presumimos que você conheça o Rails.<!--pt_BR-->
<!--pt_BR-->
Para uma imersão mais profunda em como o Loco funciona, incluindo guias detalhados, exemplos e referências da API, confira nosso [site de documentação](https://loco.rs).<!--pt_BR-->
<!--pt_BR-->
<!--pt_BR-->
## Recursos do Loco:<!--pt_BR-->
<!--pt_BR-->
* `Convenção sobre Configuração:` Semelhante ao Ruby on Rails, o Loco enfatiza simplicidade e produtividade ao reduzir a necessidade de código boilerplate. Ele utiliza padrões sensatos, permitindo que os desenvolvedores se concentrem em escrever a lógica de negócios em vez de perder tempo com configuração.<!--pt_BR-->
<!--pt_BR-->
* `Desenvolvimento Rápido:` Com o objetivo de alta produtividade para o desenvolvedor, o design do Loco se concentra em reduzir código boilerplate e fornecer APIs intuitivas, permitindo que os desenvolvedores iteren rapidamente e construam protótipos com esforço mínimo.<!--pt_BR-->
<!--pt_BR-->
* `Integração ORM:` Modele seu negócio com entidades robustas, eliminando a necessidade de escrever SQL. Defina relacionamentos, validações e lógica personalizada diretamente em suas entidades para melhorar a manutenção e escalabilidade.<!--pt_BR-->
<!--pt_BR-->
* `Controladores:` Manipule os parâmetros de solicitações web, corpo, validação e renderize uma resposta que é consciente do conteúdo. Usamos Axum para o melhor desempenho, simplicidade e extensibilidade. Os controladores também permitem que você construa facilmente middlewares, que podem ser usados para adicionar lógica como autenticação, registro ou tratamento de erros antes de passar as solicitações para as ações principais do controlador.<!--pt_BR-->
<!--pt_BR-->
* `Views:` O Loco pode se integrar com mecanismos de template para gerar conteúdo HTML dinâmico a partir de templates.<!--pt_BR-->
<!--pt_BR-->
* `Trabalhos em segundo plano:` Realize trabalhos intensivos de computação ou I/O em segundo plano com uma fila baseada em Redis ou com threads. Implementar um trabalhador é tão simples quanto implementar uma função de execução para o trait Worker.<!--pt_BR-->
<!--pt_BR-->
* `Scheduler:` Simplifica o tradicional e frequentemente complicado sistema crontab, tornando mais fácil e elegante agendar tarefas ou scripts shell.<!--pt_BR-->
<!--pt_BR-->
* `Mailers:` Um mailer entregará e-mails em segundo plano usando a infraestrutura de trabalhador existente do loco. Tudo será transparente para você.<!--pt_BR-->
<!--pt_BR-->
* `Armazenamento:` No Armazenamento do Loco, facilitamos o trabalho com arquivos por meio de várias operações. O armazenamento pode ser em memória, no disco ou utilizar serviços em nuvem, como AWS S3, GCP e Azure.<!--pt_BR-->
<!--pt_BR-->
* `Cache:` O Loco fornece uma camada de cache para melhorar o desempenho da aplicação armazenando dados acessados frequentemente.<!--pt_BR-->
<!--pt_BR-->
Para ver mais recursos do Loco, confira nosso [site de documentação](https://loco.rs/docs/getting-started/tour/).<!--pt_BR-->
<!--pt_BR-->
<!--pt_BR-->
<!--pt_BR-->
## Começando<!--pt_BR-->
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> --><!--pt_BR-->
```sh<!--pt_BR-->
cargo install loco<!--pt_BR-->
cargo install sea-orm-cli # Only when DB is needed<!--pt_BR-->
```<!--pt_BR-->
<!-- </snip> --><!--pt_BR-->
<!--pt_BR-->
Agora você pode criar seu novo aplicativo (escolha "`SaaS` app").<!--pt_BR-->
<!--pt_BR-->
<!--pt_BR-->
<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> --><!--pt_BR-->
```sh<!--pt_BR-->
❯ loco new<!--pt_BR-->
✔ ❯ App name? · myapp<!--pt_BR-->
✔ ❯ What would you like to build? · Saas App with client side rendering<!--pt_BR-->
✔ ❯ Select a DB Provider · Sqlite<!--pt_BR-->
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)<!--pt_BR-->
<!--pt_BR-->
🚂 Loco app generated successfully in:<!--pt_BR-->
myapp/<!--pt_BR-->
<!--pt_BR-->
- assets: You've selected `clientside` for your asset serving configuration.<!--pt_BR-->
<!--pt_BR-->
Next step, build your frontend:<!--pt_BR-->
  $ cd frontend/<!--pt_BR-->
  $ npm install && npm run build<!--pt_BR-->
```<!--pt_BR-->
<!-- </snip> --><!--pt_BR-->
<!--pt_BR-->
 Agora execute `cd` no seu `myapp` e inicie seu aplicativo:<!--pt_BR-->
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> --><!--pt_BR-->
```sh<!--pt_BR-->
$ cargo loco start<!--pt_BR-->
<!--pt_BR-->
                      ▄     ▀<!--pt_BR-->
                                ▀  ▄<!--pt_BR-->
                  ▄       ▀     ▄  ▄ ▄▀<!--pt_BR-->
                                    ▄ ▀▄▄<!--pt_BR-->
                        ▄     ▀    ▀  ▀▄▀█▄<!--pt_BR-->
                                          ▀█▄<!--pt_BR-->
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█<!--pt_BR-->
██████  █████   ███ █████   ███ █████   ███ ▀█<!--pt_BR-->
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄<!--pt_BR-->
██████  █████   ███ █████       █████   ███ ████▄<!--pt_BR-->
██████  █████   ███ █████   ▄▄▄ █████   ███ █████<!--pt_BR-->
██████  █████   ███  ████   ███ █████   ███ ████▀<!--pt_BR-->
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀<!--pt_BR-->
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀<!--pt_BR-->
                https://loco.rs<!--pt_BR-->
<!--pt_BR-->
listening on port 5150<!--pt_BR-->
```<!--pt_BR-->
<!-- </snip> --><!--pt_BR-->
<!--pt_BR-->
## Impulsionado pelo Loco<!--pt_BR-->
+ [SpectralOps](https://spectralops.io) - vários serviços impulsionados pelo framework Loco<!--pt_BR-->
+ [Nativish](https://nativi.sh) - backend do aplicativo impulsionado pelo framework Loco<!--pt_BR-->
<!--pt_BR-->
## Contribuidores ✨<!--pt_BR-->
Agradecimentos a essas pessoas maravilhosas:<!--pt_BR-->
<!--pt_BR-->
<a href="https://github.com/loco-rs/loco/graphs/contributors"><!--pt_BR-->
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" /><!--pt_BR-->
</a><!--pt_BR-->
 <div align="center"><!--zh_CN-->
<!--zh_CN-->
   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/><!--zh_CN-->
<!--zh_CN-->
   <h1>Loco</h1><!--zh_CN-->
<!--zh_CN-->
<!--zh_CN-->
   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)<!--zh_CN-->
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)<!--zh_CN-->
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)<!--zh_CN-->
<!--zh_CN-->
 </div><!--zh_CN-->
<!--zh_CN-->
[English](./README.md) · 中文 · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md)<!--zh_CN-->
<!--zh_CN-->
Loco 是一个用 Rust 编写的 Web 框架，类似于 Rails。Loco 提供快速构建 Web 应用的功能，并且允许创建自定义任务，可以通过 CLI 运行。<!--zh_CN-->
<!--zh_CN-->
## 特性<!--zh_CN-->
<!--zh_CN-->
- **简单的 API**: 使用 Rust 的强类型系统确保安全性和可靠性。<!--zh_CN-->
- **快速开发**: 提供快速构建 Web 应用的工具和模板。<!--zh_CN-->
- **CLI 支持**: 可以创建和运行自定义 CLI 任务。<!--zh_CN-->
- **灵活性**: 支持自定义配置和扩展。<!--zh_CN-->
<!--zh_CN-->
## 安装<!--zh_CN-->
<!--zh_CN-->
通过 Cargo 安装 Loco:<!--zh_CN-->
<!--zh_CN-->
```sh<!--zh_CN-->
cargo install loco<!--zh_CN-->
```<!--zh_CN-->
<!--zh_CN-->
## 快速开始<!--zh_CN-->
<!--zh_CN-->
创建一个新的 Loco 项目:<!--zh_CN-->
<!--zh_CN-->
```sh<!--zh_CN-->
loco new my_project<!--zh_CN-->
cd my_project<!--zh_CN-->
```<!--zh_CN-->
<!--zh_CN-->
启动开发服务器:<!--zh_CN-->
<!--zh_CN-->
```sh<!--zh_CN-->
cargo loco start<!--zh_CN-->
```<!--zh_CN-->
<!--zh_CN-->
## 贡献<!--zh_CN-->
<!--zh_CN-->
欢迎对 Loco 的贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解更多信息。<!--zh_CN-->
<!--zh_CN-->
## 许可证<!--zh_CN-->
<!--zh_CN-->
Loco 在 MIT 许可证下发布。详情请参阅 [LICENSE](LICENSE)。<!--zh_CN-->
<!--zh_CN-->
---<!--zh_CN-->
<!--zh_CN-->
For more details, you can visit the [original README file](https://github.com/loco-rs/loco/blob/master/README.md).<!--zh_CN-->
