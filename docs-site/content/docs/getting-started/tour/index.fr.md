+++
title = "Aperçu rapide"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++

<img style="width:100%; max-width:640px" src="tour.png"/>
<br/>
<br/>
<br/>
Créons un blog coté serveur sur Loco en quelques minutes. Commençons par installer `loco-cli` et `sea-orm-cli`:

<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco-cli
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->


 Vous pouvez maintenant créer votre nouvelle application (choisissez "`SaaS` app").

 ```sh
 ❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokyo async tasks)
✔ ❯ Select an asset serving configuration · Client (configures assets for frontend serving)

 🚂 Loco app generated successfully in:
 myapp/
 ```

Si vous sélectionnez tous les paramètres par défaut, vous aurez:

* `sqlite` pour la base de données. Découvrez les types de bases de données dans [Sqlite vs Postgres](@/docs/the-app/models.md#sqlite-vs-postgres) dans la section _models_ .
* `async` pour les _workers_ en arrière-plan. En savoir plus sur la configuration des _workers_  [async vs queue](@/docs/processing/workers.md#async-vs-queue) dans la section _workers_ .
* `Client` configuration pour la diffusion des ressources. Cela signifie que votre backend servira d'API.


 Maintenant, faites `cd` dans votre `myapp` et démarrez votre application en exécutant `cargo loco start`:

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


 <div class="infobox">
 Vous n'êtes pas obligé d'exécuter via `cargo` mais en développement, c'est fortement
recommandé. Si vous compilez avec `--release`, votre binaire contiendra tout
y compris votre code. Ainsi `cargo` ou Rust ne seront plus nécessaire. </div>

## Ajouter une API de type CRUD

Nous avons une application SaaS de base avec une authentification utilisateur générée pour nous. Faisons-en un backend de blog en ajoutant un `post` et une API CRUD complète à l'aide de `scaffold` :

```sh
$ cargo loco generate scaffold post title:string content:text

  :
  :
added: "src/controllers/post.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/post.rs"
injected: "tests/requests/mod.rs"
* Migration for `post` added! You can now apply it with `$ cargo loco db migrate`.
* A test for model `posts` was added. Run with `cargo test`.
* Controller `post` was added successfully.
* Tests for controller `post` was added successfully. Run `cargo test`.
```

Votre base de données a été migrée et le modèle, les entités et un contrôleur CRUD complet ont été générés automatiquement.

Redémarrez votre application :
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

Ensuite, essayez d’ajouter un `post` avec `curl`:

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "title": "Your Title",
  "content": "Your Content xxx"
}' localhost:5150/posts
```

Vous pouvez lister vos publications (posts):

```sh
$ curl localhost:5150/posts
```

Pour ceux qui comptent -- les commandes pour créer un backend de blog étaient:

1. `cargo install loco-cli`
2. `cargo install sea-orm-cli`
3. `loco new`
4. `cargo loco generate scaffold post title:string content:text`

Voilà! Profitez de votre balade avec `loco` 🚂

## Vérifions l'authentification SaaS

L'application Saas générée contient une suite d’authentification entièrement fonctionnelle, basée sur les JWT.

### Enregistrer un nouvel utilisateur

Le point de terminaison `/api/auth/register` crée un nouvel utilisateur dans la base de données avec un `email_verification_token` pour la vérification du compte. Un e-mail de bienvenue est envoyé à l'utilisateur avec un lien de vérification.

```sh
$ curl --location '127.0.0.1:5150/api/auth/register' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "name": "Loco user",
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

Pour des raisons de sécurité, si l'utilisateur est déjà enregistré, aucun nouvel utilisateur n'est créé et un statut 200 est renvoyé sans exposer les détails de l'e-mail de l'utilisateur.

### Login

Après avoir enregistré un nouvel utilisateur, utilisez la requête suivante pour vous connecter:

```sh
$ curl --location '127.0.0.1:5150/api/auth/login' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

La réponse inclut un Token (jeton) JWT pour l’authentification, l’ID utilisateur, le nom et l’état de vérification.

```sh
{
    "token": "...",
    "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
    "name": "Loco user",
    "claims": null
    "is_verified": false
}
```

Dans votre application côté client, vous enregistrez ce jeton JWT et effectuez les requêtes suivantes avec le jeton en utilisant _bearer token_ (voir ci-dessous) afin que les requêtes soient authentifiées.

### Obtenir l'utilisateur actuel

Ce point de terminaison est protégé par un middleware d'authentification. Nous utiliserons le jeton que nous avons obtenu précédemment pour effectuer une requête avec la technique _bearer token_ (remplacez `TOKEN` par le jeton JWT que vous avez obtenu précédemment):

```sh
$ curl --location --request GET '127.0.0.1:5150/api/user/current' \
     --header 'Content-Type: application/json' \
     --header 'Authorization: Bearer TOKEN'
```

Voilà votre première demande authentifiée !

Consultez le code source de `controllers/auth.rs` pour voir comment utiliser le middleware d'authentification dans vos propres contrôleurs.
