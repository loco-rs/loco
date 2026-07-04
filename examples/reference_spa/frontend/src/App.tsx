import { Link, Outlet, useNavigate } from 'react-router'
import { clearToken, getToken } from './auth/token'

export function App() {
  const navigate = useNavigate()
  const isAuthenticated = getToken() !== null

  function handleLogout() {
    clearToken()
    navigate('/login')
  }

  return (
    <div>
      <nav>
        <Link to="/posts">Posts</Link>
        {isAuthenticated && (
          <button type="button" onClick={handleLogout}>
            Logout
          </button>
        )}
      </nav>
      <Outlet />
    </div>
  )
}
