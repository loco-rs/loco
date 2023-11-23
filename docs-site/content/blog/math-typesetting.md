+++
title = "Math Typesetting"
description = "Introducing Doks, a Hugo theme helping you build modern documentation websites that are secure, fast, and SEO-ready — by default."
date = 2021-04-08T09:19:42+00:00
updated = 2021-04-08T09:19:42+00:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["Public"]

[extra]
lead = "Mathematical notation in a project can be enabled by using third party JavaScript libraries."
math = true
+++


In this example we will be using [KaTeX](https://katex.org/)

- Create a macro under `/template/macros/math.html` with a macro named `math`.
- Within this macro reference the [Auto-render Extension](https://katex.org/docs/autorender.html) or host these scripts locally.
- Import the macro in your templates like so:  

```bash
{% import 'macros/math.html' as macros %}
{% if page.extra.math or section.extra.math or config.extra.math %}
{{ macros::math() }}
{% endif %}
```

- To enable KaTex globally set the parameter `extra.math` to `true` in a project's configuration
- To enable KaTex on a per page basis include the parameter `extra.math = true` in content files

**Note:** 

1. The MathJax library is the other optional choice, and you can set the parameter `extra.library` to `mathjax` in a project's configuration
2. Use the online reference of [Supported TeX Functions](https://katex.org/docs/supported.html)

### Examples

<p>
Inline math: \(\varphi = \dfrac{1+\sqrt5}{2}= 1.6180339887…\) 
</p>

Block math:
$$
 \varphi = 1+\frac{1} {1+\frac{1} {1+\frac{1} {1+\cdots} } } 
$$
