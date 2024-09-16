+++
title = "Creating Frontend Website Using Angular"
description = "Setting up a Loco app for serving an Angular clientside app is easy. Learn how to configure and set up a full-stack Angular app with Loco."
date = 2024-01-25T18:03:52+01:00
updated = 2024-01-25T18:03:52+01:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["LimpidCrypto"]

+++

## Overview

1. Create new SaaS project
2. Edit `.devcontainer/Dockerfile`
3. Reopen the project in the Dev Container
4. Delete frontend directory
5. Generate new Angular frontend
6. Build frontend
7. Edit `config/development.yml`
8. Start Loco

## Create new SaaS project

1. Run `loco new` to create a new project
2. Navigate through the instructions until you reach the point where to decide what type of project to create
3. Select "SaaS app (with DB and user auth)"

## Edit ".devcontainer/Dockerfile"

1. Open `.devcontainer/Dockerfile`
2. Replace the content with the following:

```Dockerfile
FROM mcr.microsoft.com/vscode/devcontainers/rust:0-1

# Install postgresql-client and sea-orm-cli
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends postgresql-client \
    && cargo install sea-orm-cli \
    && chown -R vscode /usr/local/cargo

# Install Node.js and npm
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs
# Install Angular CLI
RUN npm install -g @angular/cli

COPY .env /.env
```

The Dockerfile will provide you with everything you need to develop a Loco app with an Angular frontend.

## Reopen the project in the Dev Container

With VSCode it is super easy to reopen and run the project in a Dev Container.

1. Press `Crtl + Shift + P`
2. Select `Dev Containers: Repopen in Container`
3. VSCode will open the project in the dev container. This can take a while when it is built for the first time.
4. Delete the existing `frontend` directory

Loco comes with a Vite React frontend. We can delete the whole directory because the Angular CLI will set up everything we need

## Generate new Angular frontend

1. From the project root execute `ng new frontend` to create a new Angular project
2. Navigate through the instructions

## Build frontend

1. Run `ng build` to build the Angular frontend

## Edit "config/development.yml"

As you may have noticed Angular has built the frontend into `frontend/dist/frontend/browser`. We now need to configure Loco to access the built frontend from there.

1. Open `config/development.yml`
2. Set the configs to the frontend build path:

   a. `server.middlewares.static.folder.path: "frontend/dist/frontend/browser"`

   b. `server.middlewares.static.fallback: "frontend/dist/frontend/browser/index.html"`

## Start Loco

1. Start Loco with `cargo loco start`
2. Open http://localhost:5150/

You should now see the Angular starter Website :smile:
