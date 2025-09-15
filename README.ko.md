 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Loco에 오신 것을 환영합니다</h1>

   <h3>
   🚂 Loco는 Rust on Rails입니다.
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · 한국어 · [Русский](./README.ru.md) · [Español](./README.es.md)


## Loco란?
`Loco`는 Rails에서 강한 영감을 받았습니다. Rails와 Rust를 모두 알고 계신다면 친숙하게 느껴지실 것이며, Rails만 알고 Rust를 처음 접하시는 분들에게도 Loco는 새롭게 다가올 것입니다. 참고로, Rails에 대한 사전 지식은 필수가 아닙니다.

Loco의 작동 방식에 대해 더 자세히 알아보려면 가이드, 예제, API 참조를 포함한 [문서 웹사이트](https://loco.rs)를 확인해보세요.

## Loco의 주요 기능:

* `설정보다 관습`: Ruby on Rails와 유사하게, Loco는 상용구 코드의 필요성을 줄임으로써 단순성과 생산성을 강조합니다. 합리적인 기본값을 사용하여 개발자가 설정보다는 비즈니스 로직 작성에 집중할 수 있게 합니다.

* `빠른 개발`: 높은 개발자 생산성을 목표로 하며, Loco의 설계는 상용구 코드를 줄이고 직관적인 API를 제공하여 개발자가 최소한의 노력으로 빠르게 반복하고 프로토타입을 구축할 수 있도록 합니다.

* `ORM 통합`: SQL 작성 없이 비즈니스를 강력한 엔티티로 모델링합니다. 관계, 유효성 검사, 사용자 정의 로직을 엔티티에 직접 정의하여 유지보수성과 확장성을 향상시킵니다.

* `컨트롤러`: 웹 요청 매개변수, 본문, 유효성 검사를 처리하고 컨텐츠를 인식하는 응답을 렌더링합니다. 최고의 성능, 단순성, 확장성을 위해 Axum을 사용합니다. 또한 컨트롤러를 통해 인증, 로깅, 오류 처리와 같은 로직을 추가할 수 있는 미들웨어를 쉽게 구축할 수 있습니다.

* `뷰`: Loco는 템플릿에서 동적 HTML 콘텐츠를 생성하기 위해 템플릿 엔진과 통합할 수 있습니다.

* `백그라운드 작업`: Redis 기반 큐 또는 스레드를 사용하여 계산이나 I/O 집약적인 작업을 백그라운드에서 수행합니다. Worker 트레이트에 대한 perform 함수를 구현하는 것만으로도 워커를 구현할 수 있습니다.

* `스케줄러`: 전통적이고 번거로운 crontab 시스템을 단순화하여 작업이나 셸 스크립트를 더 쉽고 우아하게 예약할 수 있습니다.

* `메일러`: 메일러는 기존 loco 백그라운드 워커 인프라를 사용하여 이메일을 백그라운드에서 전달합니다. 모든 과정이 매끄럽게 처리됩니다.

* `스토리지`: Loco 스토리지는 여러 작업을 통해 파일 작업을 용이하게 합니다. 메모리 내, 디스크, AWS S3, GCP, Azure와 같은 클라우드 서비스를 사용할 수 있습니다.

* `캐시`: Loco는 자주 접근하는 데이터를 저장하여 애플리케이션 성능을 향상시키는 캐시 레이어를 제공합니다.

더 많은 Loco 기능을 보려면 [문서 웹사이트](https://loco.rs/docs/getting-started/tour/)를 확인하세요.


## 시작하기
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

이제 새로운 앱을 만들 수 있습니다 ("`SaaS 앱`" 선택).


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

이제 `myapp` 디렉토리로 이동하여 앱을 시작하세요:

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

## Loco 사용 사례
+ [SpectralOps](https://spectralops.io) - Loco 프레임워크로 구동되는 다양한 서비스
+ [Nativish](https://nativi.sh) - Loco 프레임워크로 구동되는 앱 백엔드

## 기여자 ✨
이 멋진 분들께 감사드립니다:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
