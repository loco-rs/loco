# Frontend

## Batteries included

- [TypeScript](https://www.typescriptlang.org/): A typed superset of JavaScript
- [Vite](https://vitejs.dev/): A fast frontend build tool
- [React](https://reactjs.org/): A JavaScript library for building user interfaces
- [React Router](https://reactrouter.com/): Client-side routing
- [TanStack Query](https://tanstack.com/query/latest): Data fetching and caching

# Development

To get started with the development of the frontend, follow these steps:

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

This will start the development frontend server serving via Vite.

### 3. Build The application

To build your application run the following command:

```sh
pnpm build
```

After the build the `dist` folder is ready to be served by loco. Run `cargo loco start` and the frontend application will be served via Loco.
