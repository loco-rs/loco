import { Link, Outlet } from 'react-router'

export function App() {
  return (
    <div>
      <nav>
        <Link to="/posts">Posts</Link>
      </nav>
      <Outlet />
    </div>
  )
}
