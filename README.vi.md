<div align="center">

  <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

  <h1>Chào mừng đến với Loco</h1>

  <h3>
  <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco là Rust trên Rails.
<!--</snip> -->
  </h3>

  [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
  [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
  [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

</div>


[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md) · Vietnamese


## Loco là gì?
`Loco` được lấy cảm hứng mạnh mẽ từ Rails. Nếu bạn biết Rails và Rust, bạn sẽ cảm thấy quen thuộc. Nếu bạn chỉ biết Rails và mới làm quen với Rust, bạn sẽ thấy Loco rất thú vị. Chúng tôi không giả định rằng bạn phải biết Rails.

Để tìm hiểu sâu hơn về cách Loco hoạt động, bao gồm hướng dẫn chi tiết, ví dụ và tài liệu tham khảo API, hãy xem [trang tài liệu](https://loco.rs) của chúng tôi.


## Tính năng của Loco:

* `Ưu tiên Quy ước hơn Cấu hình:` Tương tự như Ruby on Rails, Loco nhấn mạnh sự đơn giản và năng suất bằng cách giảm thiểu nhu cầu code boilerplate. Framework sử dụng các giá trị mặc định hợp lý, cho phép các developer tập trung vào việc viết logic nghiệp vụ thay vì dành thời gian cho cấu hình.

* `Phát triển Nhanh chóng:` Nhắm đến năng suất cao cho developer, thiết kế của Loco tập trung vào việc giảm code boilerplate và cung cấp các API trực quan, cho phép developer lặp lại nhanh chóng và xây dựng prototype với nỗ lực tối thiểu.

* `Tích hợp ORM:` Mô hình hóa nghiệp vụ của bạn với các entity mạnh mẽ, loại bỏ nhu cầu viết SQL. Định nghĩa quan hệ, validation và logic tùy chỉnh trực tiếp trên các entity của bạn để tăng cường khả năng bảo trì và mở rộng.

* `Controllers:` Xử lý các tham số request web, body, validation và render response nhận biết nội dung. Chúng tôi sử dụng Axum để có hiệu suất tốt nhất, đơn giản và dễ mở rộng. Controllers cũng cho phép bạn dễ dàng xây dựng các middleware, có thể được sử dụng để thêm logic như xác thực, logging hoặc xử lý lỗi trước khi chuyển request đến các action controller chính.

* `Views:` Loco có thể tích hợp với các template engine để tạo nội dung HTML động từ templates.

* `Background Jobs:` Thực hiện các công việc tính toán hoặc I/O intensive ở chế độ nền với hàng đợi được hỗ trợ bởi Redis, hoặc với threads. Việc triển khai một worker đơn giản như việc triển khai một hàm perform cho trait Worker.

* `Scheduler:` Đơn giản hóa hệ thống crontab truyền thống, thường cồng kềnh, giúp việc lên lịch các task hoặc shell script dễ dàng và tinh tế hơn.

* `Mailers:` Một mailer sẽ gửi email ở chế độ nền sử dụng cơ sở hạ tầng background worker hiện có của loco. Mọi thứ sẽ liền mạch với bạn.

* `Storage:` Trong Loco Storage, chúng tôi hỗ trợ làm việc với file thông qua nhiều thao tác. Storage có thể lưu trong bộ nhớ, trên đĩa hoặc sử dụng các dịch vụ cloud như AWS S3, GCP và Azure.

* `Cache:` Loco cung cấp một lớp cache để cải thiện hiệu suất ứng dụng bằng cách lưu trữ dữ liệu được truy cập thường xuyên.

Để xem thêm các tính năng của Loco, hãy xem [trang tài liệu](https://loco.rs/docs/getting-started/tour/) của chúng tôi.



## Bắt đầu
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Chỉ khi cần DB
```
<!-- </snip> -->

Bây giờ bạn có thể tạo ứng dụng mới của mình (chọn ứng dụng "`SaaS`").


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

 Bây giờ hãy `cd` vào thư mục `myapp` và khởi động ứng dụng của bạn:
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

## Được xây dựng bằng Loco
+ [SpectralOps](https://spectralops.io) - nhiều dịch vụ được xây dựng bằng Loco
  framework
+ [Nativish](https://nativi.sh) - backend ứng dụng được xây dựng bằng Loco framework

## Contributors ✨
Cảm ơn những người tuyệt vời này:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
