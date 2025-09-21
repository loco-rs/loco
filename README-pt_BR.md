 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Bem-vindo ao Loco</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco is Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · Portuguese (Brazil) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md)


## O que é o Loco?
`Loco` é fortemente inspirado no Rails. Se você conhece Rails e Rust, se sentirá em casa. Se você só conhece Rails e é novo em Rust, achará o Loco refrescante. Não presumimos que você conheça o Rails.

Para uma imersão mais profunda em como o Loco funciona, incluindo guias detalhados, exemplos e referências da API, confira nosso [site de documentação](https://loco.rs).


## Recursos do Loco:

* `Convenção sobre Configuração:` Semelhante ao Ruby on Rails, o Loco enfatiza simplicidade e produtividade ao reduzir a necessidade de código boilerplate. Ele utiliza padrões sensatos, permitindo que os desenvolvedores se concentrem em escrever a lógica de negócios em vez de perder tempo com configuração.

* `Desenvolvimento Rápido:` Com o objetivo de alta produtividade para o desenvolvedor, o design do Loco se concentra em reduzir código boilerplate e fornecer APIs intuitivas, permitindo que os desenvolvedores iteren rapidamente e construam protótipos com esforço mínimo.

* `Integração ORM:` Modele seu negócio com entidades robustas, eliminando a necessidade de escrever SQL. Defina relacionamentos, validações e lógica personalizada diretamente em suas entidades para melhorar a manutenção e escalabilidade.

* `Controladores:` Manipule os parâmetros de solicitações web, corpo, validação e renderize uma resposta que é consciente do conteúdo. Usamos Axum para o melhor desempenho, simplicidade e extensibilidade. Os controladores também permitem que você construa facilmente middlewares, que podem ser usados para adicionar lógica como autenticação, registro ou tratamento de erros antes de passar as solicitações para as ações principais do controlador.

* `Views:` O Loco pode se integrar com mecanismos de template para gerar conteúdo HTML dinâmico a partir de templates.

* `Trabalhos em segundo plano:` Realize trabalhos intensivos de computação ou I/O em segundo plano com uma fila baseada em Redis ou com threads. Implementar um trabalhador é tão simples quanto implementar uma função de execução para o trait Worker.

* `Scheduler:` Simplifica o tradicional e frequentemente complicado sistema crontab, tornando mais fácil e elegante agendar tarefas ou scripts shell.

* `Mailers:` Um mailer entregará e-mails em segundo plano usando a infraestrutura de trabalhador existente do loco. Tudo será transparente para você.

* `Armazenamento:` No Armazenamento do Loco, facilitamos o trabalho com arquivos por meio de várias operações. O armazenamento pode ser em memória, no disco ou utilizar serviços em nuvem, como AWS S3, GCP e Azure.

* `Cache:` O Loco fornece uma camada de cache para melhorar o desempenho da aplicação armazenando dados acessados frequentemente.

Para ver mais recursos do Loco, confira nosso [site de documentação](https://loco.rs/docs/getting-started/tour/).



## Começando
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Agora você pode criar seu novo aplicativo (escolha "`SaaS` app").


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

 Agora execute `cd` no seu `myapp` e inicie seu aplicativo:
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

## Impulsionado pelo Loco
+ [SpectralOps](https://spectralops.io) - vários serviços impulsionados pelo framework Loco
+ [Nativish](https://nativi.sh) - backend do aplicativo impulsionado pelo framework Loco

## Contribuidores ✨
Agradecimentos a essas pessoas maravilhosas:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
