 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>欢迎来到 Loco</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco is Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · 中文 · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md) · [Vietnamese](./README.vi.md) · [العربية](./README.ar.md)

## 什么是 Loco？

`Loco` 深受 Rails 启发。如果你熟悉 Rails 和 Rust，使用 Loco 会得心应手。如果你熟悉 Rails 但刚接触 Rust，Loco 也会让你耳目一新。使用 Loco 并不要求你预先了解 Rails。

深入了解 Loco 的工作原理，包括详细指南、示例和 API 参考，请访问我们的[文档网站](https://loco.rs)。

## Loco 的特性

* `约定优于配置：` 与 Ruby on Rails 类似，Loco 注重简洁与开发效率，尽量减少样板代码。它提供合理的默认配置，让开发者把精力放在业务逻辑上，而非繁琐的配置工作上。

* `快速开发：` Loco 通过减少样板代码和提供直观的 API 来提高开发效率，使开发者能够快速迭代，并以更少的工作量构建原型。

* `ORM 集成：` 使用可靠的实体模型描述业务，无需编写 SQL。可以直接在实体上定义关系、验证规则和自定义逻辑，从而提升可维护性和可扩展性。

* `控制器（Controllers）：` 处理 Web 请求参数、请求体和验证，并渲染与内容类型相匹配的响应。我们使用 Axum，以获得最佳的性能、简洁性和可扩展性。控制器还让你能够轻松构建中间件，在请求传递给主要的控制器处理函数之前，加入身份验证、日志记录或错误处理等逻辑。

* `视图（Views）：` Loco 可以与模板引擎集成，根据模板生成动态 HTML 内容。

* `后台作业（Background Jobs）：` 借助 Redis 支持的队列或线程，在后台执行计算密集型或 I/O 密集型作业。要实现一个 worker，只需为 `Worker` trait 实现 `perform` 函数。

* `调度器（Scheduler）：` 简化传统而繁琐的 crontab 系统，让任务或 shell 脚本的定时执行更简单、更优雅。

* `邮件（Mailers）：` mailer 会利用 Loco 现有的后台 worker 基础设施，在后台发送电子邮件，整个过程都能无缝完成。

* `存储（Storage）：` Loco Storage 提供多种文件操作，简化文件处理。文件可以存储在内存或磁盘中，也可以使用 AWS S3、GCP 和 Azure 等云服务。

* `缓存（Cache）：` Loco 提供缓存层，通过存储频繁访问的数据来提升应用性能。

了解 Loco 的更多特性，请访问我们的[文档网站](https://loco.rs/docs/getting-started/tour/)。

## 快速开始
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

现在可以创建新应用了（选择“`SaaS` 应用”）。

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

现在进入 `myapp` 目录并启动应用：
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

## 由 Loco 驱动的应用

* [SpectralOps](https://spectralops.io) - 多项服务由 Loco 框架驱动
* [Nativish](https://nativi.sh) - 应用后端由 Loco 框架驱动

## 贡献者 ✨

感谢这些优秀的贡献者：

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
