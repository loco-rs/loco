import { Link } from 'react-router'

export function Home() {
  return (
    <div>
      <h1>Reference SPA — Home</h1>
      <nav>
        <ul>
          {/* scaffold:nav */}
          <li><Link to="/posts">Posts</Link></li>
        </ul>
      </nav>
      <p>
        <Link to="/login">Log in</Link> or{' '}
        <Link to="/register">sign up</Link> to get started.
      </p>
    </div>
  )
}
