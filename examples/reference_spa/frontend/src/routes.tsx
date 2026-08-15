import { createBrowserRouter } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { RequireAuth } from './auth/RequireAuth'
import { Home } from './pages/Home'
// scaffold:imports
import { Edit as PostsEdit } from './pages/posts/Edit'
import { List as PostsList } from './pages/posts/List'
import { New as PostsNew } from './pages/posts/New'
import { Show as PostsShow } from './pages/posts/Show'

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
          // scaffold:routes
          { path: 'posts', element: <PostsList /> },
          { path: 'posts/new', element: <PostsNew /> },
          { path: 'posts/:id', element: <PostsShow /> },
          { path: 'posts/:id/edit', element: <PostsEdit /> },
        ],
      },
    ],
  },
])
