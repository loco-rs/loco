 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Loco'ya Hoş Geldiniz</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco, Rust için Rails'tir.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md) · [Vietnamese](./README.vi.md) · [العربية](./README.ar.md) · [Bahasa Indonesia](./README.id.md) · Türkçe

## Loco Nedir?

`Loco`, Rails'ten büyük ölçüde esinlenmiştir. Rails ve Rust biliyorsanız kendinizi evinizde hissedersiniz. Yalnızca Rails biliyor ve Rust'a yeniyseniz, Loco size ferahlatıcı gelecektir. Rails bildiğinizi varsaymıyoruz.

Loco'nun nasıl çalıştığına dair daha derinlemesine bilgi edinmek için ayrıntılı kılavuzlar, örnekler ve API referansları da dahil olmak üzere [dokümantasyon sitemize](https://loco.rs) göz atın.

## Loco'nun Özellikleri

* `Yapılandırma Yerine Gelenek:` Ruby on Rails'e benzer şekilde Loco, standart kod (boilerplate) ihtiyacını azaltarak sadeliği ve üretkenliği vurgular. Makul varsayılanlar kullanır ve geliştiricilerin yapılandırmaya zaman harcamak yerine iş mantığı yazmaya odaklanmasını sağlar.

* `Hızlı Geliştirme:` Yüksek geliştirici üretkenliğini hedefleyen Loco'nun tasarımı, standart kodu azaltmaya ve sezgisel API'ler sunmaya odaklanır; böylece geliştiriciler hızlıca yineleme yapabilir ve minimum çabayla prototipler oluşturabilir.

* `ORM Entegrasyonu:` İşinizi sağlam varlıklarla modelleyin; SQL yazma ihtiyacını ortadan kaldırın. İlişkileri, doğrulamayı ve özel mantığı doğrudan varlıklarınız üzerinde tanımlayarak bakım kolaylığını ve ölçeklenebilirliği artırın.

* `Controller'lar:` Web isteklerinin parametrelerini, gövdesini ve doğrulamasını ele alın; içerik farkındalığı olan bir yanıt oluşturun. En iyi performans, sadelik ve genişletilebilirlik için Axum kullanıyoruz. Controller'lar ayrıca middleware'leri kolayca oluşturmanıza olanak tanır; bunlar istekleri ana controller eylemlerine iletmeden önce kimlik doğrulama, günlükleme veya hata işleme gibi mantıklar eklemek için kullanılabilir.

* `View'lar:` Loco, şablonlardan dinamik HTML içeriği oluşturmak için şablon motorlarıyla entegre olabilir.

* `Arka Plan İşleri:` Hesaplama veya G/Ç açısından yoğun işleri, Redis destekli bir kuyruk ya da thread'ler ile arka planda gerçekleştirin. Bir worker uygulamak, `Worker` trait'i için bir `perform` fonksiyonu uygulamak kadar basittir.

* `Zamanlayıcı:` Geleneksel ve genellikle zahmetli olan crontab sistemini basitleştirir; görevleri veya shell betiklerini planlamayı daha kolay ve daha zarif hale getirir.

* `Mailer'lar:` Bir mailer, e-postaları mevcut loco arka plan worker altyapısını kullanarak arka planda gönderir. Tümü sizin için sorunsuz gerçekleşir.

* `Depolama:` Loco Storage'da, birden fazla işlem aracılığıyla dosyalarla çalışmayı kolaylaştırıyoruz. Depolama bellek içinde, diskte veya AWS S3, GCP ve Azure gibi bulut hizmetlerinde olabilir.

* `Önbellek:` Loco, sık erişilen verileri saklayarak uygulama performansını artırmak için bir önbellek katmanı sunar.

Daha fazla Loco özelliği görmek için [dokümantasyon sitemize](https://loco.rs/docs/getting-started/tour/) göz atın.

## Başlarken
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Artık yeni uygulamanızı oluşturabilirsiniz ("`SaaS` app" seçeneğini seçin).

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

Şimdi `myapp` dizinine `cd` yapın ve uygulamanızı başlatın:
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

## Loco ile Güçlendirilmiş

* [SpectralOps](https://spectralops.io) - Loco framework'ü ile güçlendirilmiş çeşitli hizmetler
* [Nativish](https://nativi.sh) - Loco framework'ü ile güçlendirilmiş uygulama arka ucu

## Katkıda Bulunanlar ✨

Bu harika insanlara teşekkürler:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
