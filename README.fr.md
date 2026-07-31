 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Loco vous souhaite la bienvenue</h1>

   <h3>
🚂 Loco c'est Rust on Rails.
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · [中文](./README-zh_CN.md) · Français · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md) · [Vietnamese](./README.vi.md) · [العربية](./README.ar.md)

## À propos de Loco
`Loco` est fortement inspiré de Rails. Si vous connaissez Rails et Rust, vous vous sentirez chez vous. Si vous ne connaissez que Rails et que vous êtes nouveau sur Rust, vous trouverez Loco rafraîchissant. Nous ne supposons pas que vous connaissez Rails.
Pour un aperçu plus approfondie du fonctionnement de Loco, y compris des guides détaillés, des exemples et des références API, consultez notre [site Web de documentation](https://loco.rs).

## Caractéristiques de Loco:

* `Convention plutôt que configuration`: Semblable à Ruby on Rails, Loco met l'accent sur la simplicité et la productivité en réduisant le besoin de code passe-partout. Il utilise des valeurs par défaut raisonnables, permettant aux développeurs de se concentrer sur l'écriture de la logique métier plutôt que de consacrer du temps à la configuration.

* `Développement rapide`: Visant une productivité élevée des développeurs, la conception de Loco se concentre sur la réduction du code passe-partout et la fourniture d'API intuitives, permettant aux développeurs d'intégrer rapidement et de créer des prototypes avec un minimum d'effort.

* `Intégration ORM`: Modélisez avec des entités robustes, éliminant le besoin d'écrire du SQL. Définissez les relations, la validation et la logique sur mesure directement sur vos entités pour une maintenabilité et une évolutivité améliorées.

* `Contrôleurs`: Gérez les paramètres et le contenu des requêtes Web, la validation des requêtes et affichez une réponse tenant compte du contenu. Nous utilisons Axum pour une meilleure performance, simplicité et extensibilité. Les contrôleurs vous permettent également de créer facilement des middlewares, qui peuvent être utilisés pour ajouter une logique telle que l'authentification, la journalisation (logging) ou la gestion des erreurs avant de transmettre les requêtes aux actions du contrôleur principal.

* `Vues`: Loco peut s'intégrer aux moteurs de _templates_ pour générer du contenu HTML dynamique à partir de modèles template.

* `Tâches en arrière-plan`: Effectuer des calculs informatiques ou d'I/O (Entrée/Sortie) intensives en arrière-plan avec une file d'attente sauvegardée Redis ou avec des threads. Implémenter un travailleur (worker) est aussi simple que d'implémenter une fonction d'exécution pour le trait Worker.

* `Scheduler`: Simplifie le système crontab traditionnel, souvent encombrant, en rendant plus facile et plus élégante la planification de tâches ou de scripts shell.

* `Mailers`: Un logiciel de messagerie enverra des e-mails en arrière-plan en utilisant l'infrastructure de travail d'arrière-plan de Loco existante. Tout se passera sans problème pour vous.

* `Stockage`: Loco Storage facilite le travail avec des fichiers via plusieurs opérations. Le stockage peut être en mémoire, sur disque ou utiliser des services cloud tels qu'AWS S3, GCP et Azure.

* `Cache :` Loco fournit une strate cache pour améliorer les performances des applications en stockant les données fréquemment consultées.

Pour en savoir plus sur les fonctionnalités de Loco, consultez notre [site Web de documentation](https://loco.rs/docs/getting-started/tour/).


## Commencez rapidement
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Vous pouvez maintenant créer votre nouvelle application (choisissez "`SaaS` app").


<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · Saas App with client side rendering
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)

🚂 Loco app generated successfully in:
myapp/

- assets: You've selected `clientside` for your asset serving configuration.

Next step, build your frontend:
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

Maintenant, faite `cd` dans votre `myapp` et démarrez votre application:

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

## Servi par Loco
+ [SpectralOps](https://spectralops.io) - divers services servi par le framework Loco
+ [Nativish](https://nativi.sh) - app backend servi par le framework Loco

## Contributeurs ✨
Merci à ces personnes formidables :

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>

