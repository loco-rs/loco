+++
title = "Quick Start"
description = "One page summary of how to start a new AdiDoks project."
date = 2021-05-01T08:20:00+00:00
updated = 2021-05-01T08:20:00+00:00
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "One page summary of how to start a new AdiDoks project."
toc = true
top = false
+++

## Requirements

Before using the theme, you need to install the [Zola](https://www.getzola.org/documentation/getting-started/installation/) ≥ 0.15.0.

## Run the Theme Directly

```bash
git clone https://github.com/aaranxu/adidoks.git
cd adidoks
zola serve
```

Visit `http://127.0.0.1:1111/` in the browser.

## Installation

Just earlier we showed you how to run the theme directly. Now we start to
install the theme in an existing site step by step.

### Step 1: Create a new zola site

```bash
zola init mysite
```

### Step 2: Install AdiDoks

Download this theme to your themes directory:

```bash
cd mysite/themes
git clone https://github.com/aaranxu/adidoks.git
```

Or install as a submodule:

```bash
cd mysite
git init  # if your project is a git repository already, ignore this command
git submodule add https://github.com/aaranxu/adidoks.git themes/adidoks
```

### Step 3: Configuration

Enable the theme in your `config.toml` in the site derectory:

```toml
theme = "adidoks"
```

Or copy the `config.toml.example` from the theme directory to your project's
root directory:

```bash
cp themes/adidoks/config.toml.example config.toml
```

### Step 4: Add new content

You can copy the content from the theme directory to your project:

```bash
cp -r themes/adidoks/content .
```

You can modify or add new posts in the `content/blog`, `content/docs` or other
content directories as needed.

### Step 5: Run the project

Just run `zola serve` in the root path of the project:

```bash
zola serve
```

AdiDoks will start the Zola development web server accessible by default at 
`http://127.0.0.1:1111`. Saved changes will live reload in the browser.
