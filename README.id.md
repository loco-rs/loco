 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Selamat datang di Loco</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco adalah Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md) · [Vietnamese](./README.vi.md) · [العربية](./README.ar.md) · Bahasa Indonesia

## Apa itu Loco?

`Loco` sangat terinspirasi oleh Rails. Jika Anda mengenal Rails dan Rust, Anda akan merasa familiar. Jika Anda hanya mengenal Rails dan baru menggunakan Rust, Loco akan terasa menyegarkan. Kami tidak berasumsi bahwa Anda sudah mengenal Rails.

Untuk memahami lebih dalam cara kerja Loco, termasuk panduan terperinci, contoh, dan referensi API, kunjungi [situs dokumentasi kami](https://loco.rs).

## Fitur Loco

* `Convention Over Configuration:` Seperti Ruby on Rails, Loco menekankan kesederhanaan dan produktivitas dengan mengurangi kebutuhan akan kode boilerplate. Loco menggunakan default yang masuk akal, sehingga developer dapat berfokus pada penulisan logika bisnis alih-alih menghabiskan waktu untuk konfigurasi.

* `Rapid Development:` Untuk mendorong produktivitas developer yang tinggi, desain Loco berfokus pada pengurangan kode boilerplate dan penyediaan API yang intuitif, sehingga developer dapat melakukan iterasi dengan cepat dan membangun prototipe dengan upaya minimal.

* `ORM Integration:` Modelkan bisnis Anda dengan entitas yang kuat tanpa perlu menulis SQL. Definisikan relasi, validasi, dan logika kustom langsung pada entitas Anda untuk meningkatkan kemudahan pemeliharaan dan skalabilitas.

* `Controllers`: Menangani parameter request web, body, validasi, dan merender response yang sesuai dengan konten. Kami menggunakan Axum untuk mendapatkan performa, kesederhanaan, dan ekstensibilitas terbaik. Controllers juga memungkinkan Anda membangun middleware dengan mudah untuk menambahkan logika seperti autentikasi, logging, atau penanganan error sebelum request diteruskan ke action controller utama.

* `Views:` Loco dapat terintegrasi dengan templating engine untuk menghasilkan konten HTML dinamis dari template.

* `Background Jobs:` Menjalankan job yang intensif dalam komputasi atau I/O di background dengan queue berbasis Redis atau menggunakan thread. Mengimplementasikan worker cukup dengan mengimplementasikan fungsi `perform` untuk trait `Worker`.

* `Scheduler:` Menyederhanakan sistem crontab tradisional yang sering kali rumit, sehingga penjadwalan task atau shell script menjadi lebih mudah dan elegan.

* `Mailers:` Mailer akan mengirim email di background menggunakan infrastruktur background worker Loco yang sudah ada. Semuanya akan berjalan mulus untuk Anda.

* `Storage:` Di Loco Storage, kami memudahkan pengelolaan file melalui berbagai operasi. Storage dapat berada di memori, di disk, atau menggunakan layanan cloud seperti AWS S3, GCP, dan Azure.

* `Cache:` Loco menyediakan layer cache untuk meningkatkan performa aplikasi dengan menyimpan data yang sering diakses.

Untuk melihat lebih banyak fitur Loco, kunjungi [situs dokumentasi kami](https://loco.rs/docs/getting-started/tour/).

## Memulai
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Sekarang Anda dapat membuat aplikasi baru (pilih aplikasi "`SaaS`").

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

Sekarang jalankan `cd` ke `myapp` dan mulai aplikasi Anda:
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

## Didukung oleh Loco

* [SpectralOps](https://spectralops.io) - berbagai layanan yang didukung oleh framework Loco
* [Nativish](https://nativi.sh) - backend aplikasi yang didukung oleh framework Loco

## Kontributor ✨

Terima kasih kepada orang-orang luar biasa berikut:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
