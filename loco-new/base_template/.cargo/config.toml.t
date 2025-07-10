[alias]
loco = "run --"
{%- if settings.os == "windows" %}
loco-tool = "run --bin tool --"
{% else %}
loco-tool = "run --"
{%- endif %}

playground = "run --example playground"

# https://github.com/rust-lang/rust/issues/141626
# (can be removed once link.exe is fixed)
[target.x86_64-pc-windows-msvc]
linker = "rust-lld"
