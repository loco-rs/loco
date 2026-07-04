import { createBrowserRouter } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { RequireAuth } from './auth/RequireAuth'
import { Home } from './pages/Home'
import { Edit } from './pages/posts/Edit'
import { List } from './pages/posts/List'
import { New } from './pages/posts/New'
import { Show } from './pages/posts/Show'

export const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    children: [
      { index: true, element: <Home /> },
      { path: 'login', element: <Login /> },
      {
        element: <RequireAuth />,
        children: [
          { path: 'posts', element: <List /> },
          { path: 'posts/new', element: <New /> },
          { path: 'posts/:id', element: <Show /> },
          { path: 'posts/:id/edit', element: <Edit /> },
        ],
      },
    ],
  },
])
