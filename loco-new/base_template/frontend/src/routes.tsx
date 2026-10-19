import { createBrowserRouter } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { Register } from './auth/Register'
import { RequireAuth } from './auth/RequireAuth'
import { Home } from './pages/Home'
// scaffold:imports

export const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    children: [
      { index: true, element: <Home /> },
      { path: 'login', element: <Login /> },
      { path: 'register', element: <Register /> },
      {
        element: <RequireAuth />,
        children: [
          // scaffold:routes
        ],
      },
    ],
  },
])
