import { useState } from "react";
import type { FormEvent } from "react";
import { Link } from "react-router";
import { ApiClientError, post } from "../api/client";

export function Register() {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [isPending, setIsPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isDone, setIsDone] = useState(false);

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError(null);
    setIsPending(true);
    try {
      // `register` answers 200 with a null body whether or not the address was
      // already taken — deliberately, so the endpoint cannot be used to probe
      // for accounts. There is nothing to read back, and the success copy has
      // to stay neutral for the same reason.
      await post<null>("/api/auth/register", { name, email, password });
      setIsDone(true);
    } catch (err) {
      setError(
        err instanceof ApiClientError ? err.message : "Failed to register",
      );
    } finally {
      setIsPending(false);
    }
  }

  if (isDone) {
    return (
      <div>
        <h1>Check your email</h1>
        <p>
          If that address is available, a verification link is on its way. You
          need to verify before you can log in.
        </p>
        <p>
          <Link to="/login">Back to log in</Link>
        </p>
      </div>
    );
  }

  return (
    <div>
      <h1>Sign up</h1>
      <form onSubmit={handleSubmit}>
        <div>
          <label htmlFor="name">Name</label>
          <input
            id="name"
            name="name"
            type="text"
            autoComplete="name"
            required
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="email">Email</label>
          <input
            id="email"
            name="email"
            type="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="password">Password</label>
          <input
            id="password"
            name="password"
            type="password"
            autoComplete="new-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </div>
        <button type="submit" disabled={isPending}>
          {isPending ? "Signing up…" : "Sign up"}
        </button>
      </form>
      {error && <p role="alert">{error}</p>}
      <p>
        Already have an account? <Link to="/login">Log in</Link>
      </p>
    </div>
  );
}
