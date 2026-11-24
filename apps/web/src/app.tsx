import { lazy, Suspense, useCallback, useState, type FormEvent } from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { api, failure, readResult, useResource, type ApiFailure, type Schema } from "./api";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Brand, ErrorNotice } from "./components/presentation";

const CustomerWorkspace = lazy(() => import("./customers").then((module) => ({ default: module.CustomerWorkspace })));

export function App() {
  const location = useLocation();
  const [signedIn, setSignedIn] = useState<Schema["SessionDetails"]>();
  const [signedOut, setSignedOut] = useState(false);
  const load = useCallback(async (signal: AbortSignal) => {
    const result = await api.GET("/api/v1/auth/me", { signal });
    return result.response.status === 401 ? null : readResult(result).data;
  }, []);
  const { state, reload } = useResource(load);
  const loginPage = (
    <main className="login-layout">
      <section className="login-card">
        <Brand />
        {!signedOut && state.status === "loading" ? (
          <p role="status">Memeriksa sesi…</p>
        ) : !signedOut && state.status === "error" ? (
          <ErrorNotice message={state.error.message} retry={reload} />
        ) : (
          <LoginForm onLogin={(value) => {
            setSignedIn(value);
            setSignedOut(false);
          }} />
        )}
      </section>
    </main>
  );

  if (!signedOut && state.status === "loading") return loginPage;
  const session = signedIn ?? (state.status === "ready" ? state.data : null);
  if (!signedOut && session) return <Routes>
    <Route path="/login" element={<ReturnToRoute />} />
    <Route path="/*" element={<Suspense fallback={<main className="workspace"><p className="panel" role="status">Memuat aplikasi…</p></main>}><CustomerWorkspace session={session} onLogout={() => {
      setSignedIn(undefined);
      setSignedOut(true);
    }} /></Suspense>} />
  </Routes>;

  return <Routes>
    <Route path="/login" element={loginPage} />
    <Route path="*" element={<Navigate to="/login" state={{ from: location }} replace />} />
  </Routes>;
}

function ReturnToRoute() {
  const location = useLocation();
  const from = (location.state as { from?: { pathname?: string; search?: string; hash?: string } } | null)?.from;
  const target = from?.pathname?.startsWith("/") && !from.pathname.startsWith("//")
    ? `${from.pathname}${from.search ?? ""}${from.hash ?? ""}`
    : "/customers";
  return <Navigate to={target} replace />;
}

function LoginForm({ onLogin }: { onLogin: (session: Schema["SessionDetails"]) => void }) {
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);

  async function login(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    setError(undefined);
    setPending(true);
    try {
      const result = await api.POST("/api/v1/auth/login", {
        body: { username: String(data.get("username")), password: String(data.get("password")) },
      });
      onLogin(readResult(result).data);
    } catch (error) {
      setError(failure(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <h1>Masuk</h1>
      <form className="stack" onSubmit={login}>
        <label>Nama pengguna<Input name="username" autoComplete="username" required maxLength={128} /></label>
        <label>Kata sandi<Input name="password" type="password" autoComplete="current-password" required maxLength={1024} /></label>
        {error && <ErrorNotice message={error.message} />}
        <Button type="submit" disabled={pending}>{pending ? "Memeriksa…" : "Masuk"}</Button>
      </form>
    </>
  );
}
