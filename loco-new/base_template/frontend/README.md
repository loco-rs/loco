# SaaS Frontend

## Batteries included

- [TypeScript](https://www.typescriptlang.org/): A typed superset of JavaScript
- [Rsbuild](https://rsbuild.dev/): A Rust-based web build tool
- [Biome](https://biomejs.dev/): A Rust-based formatter and sensible linter for the web
- [React](https://reactjs.org/): A JavaScript library for building user interfaces

If you don't like React for some reason, Rsbuild makes it easy to replace it with something else!

# Development

To get started with the development of the SaaS frontend, follow these steps:

### 1. Install Packages

Use the following command to install the required packages using pnpm:

```sh
pnpm install
```

### 2. Run in Development Mode

Once the packages are installed, run your frontend application in development mode with the following command:

```sh
pnpm dev
```

This will start the development frontend server serving via vit

### 3. Build The application

To build your application run the following command:

```sh
pnpm build
```

After the build `dist` folder is ready to served by loco. run loco `cargo loco start` and the frontend application will served via Loco