import { lazy, Suspense, useCallback, useState, type FormEvent } from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { Lock, User as UserIcon, Store } from "lucide-react";
import { api, failure, readResult, useResource, type ApiFailure, type Schema } from "./api";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { ErrorNotice } from "./components/presentation";

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
    <main className="min-h-screen bg-slate-100/70 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        <Card className="shadow-lg border-slate-200/90 overflow-hidden">
          <div className="bg-gradient-to-br from-[#0f4c3a] to-[#092e23] p-8 text-white text-center">
            <div className="inline-flex items-center justify-center h-14 w-14 rounded-2xl bg-white/10 backdrop-blur-sm shadow-inner ring-1 ring-white/20 mb-3">
              <Store className="h-7 w-7 text-white" />
            </div>
            <h1 className="text-xl font-bold tracking-tight">Keuangan Toko</h1>
            <p className="text-xs text-emerald-100/80 mt-1">Sistem Pengelolaan Piutang & Pembayaran Toko</p>
          </div>

          <CardHeader className="pt-6 pb-2 text-center">
            <CardTitle className="text-lg">Masuk ke Sistem</CardTitle>
            <CardDescription>Masukkan nama pengguna dan kata sandi Anda</CardDescription>
          </CardHeader>

          <CardContent className="p-6 pt-2">
            {!signedOut && state.status === "loading" ? (
              <div className="py-8 text-center text-sm text-slate-500 animate-pulse" role="status">
                Memeriksa sesi pengguna…
              </div>
            ) : !signedOut && state.status === "error" ? (
              <div className="py-2">
                <ErrorNotice message={state.error.message} retry={reload} />
              </div>
            ) : (
              <LoginForm
                onLogin={(value) => {
                  setSignedIn(value);
                  setSignedOut(false);
                }}
              />
            )}
          </CardContent>
        </Card>
      </div>
    </main>
  );

  if (!signedOut && state.status === "loading") return loginPage;
  const session = signedIn ?? (state.status === "ready" ? state.data : null);

  if (!signedOut && session) {
    return (
      <Routes>
        <Route path="/login" element={<ReturnToRoute passwordChangeRequired={session.must_change_password} />} />
        <Route
          path="/*"
          element={
            <Suspense
              fallback={
                <div className="min-h-screen bg-slate-50 flex items-center justify-center p-6 text-sm text-slate-500">
                  <div className="flex items-center gap-3">
                    <div className="h-5 w-5 border-2 border-emerald-700 border-t-transparent rounded-full animate-spin" />
                    <span>Memuat aplikasi…</span>
                  </div>
                </div>
              }
            >
              <CustomerWorkspace
                session={session}
                onPasswordChanged={() => setSignedIn({ ...session, must_change_password: false })}
                onLogout={() => {
                  void (async () => {
                    try {
                      readResult(await api.POST("/api/v1/auth/logout", {
                        params: { header: { "X-CSRF-Token": session.csrf_token } },
                      }));
                    } catch { /* Local sign-out still completes when the session has expired. */ }
                    setSignedIn(undefined);
                    setSignedOut(true);
                  })();
                }}
              />
            </Suspense>
          }
        />
      </Routes>
    );
  }

  return (
    <Routes>
      <Route path="/login" element={loginPage} />
      <Route path="*" element={<Navigate to="/login" state={{ from: location }} replace />} />
    </Routes>
  );
}

function ReturnToRoute({ passwordChangeRequired = false }: { passwordChangeRequired?: boolean }) {
  const location = useLocation();
  const from = (location.state as { from?: { pathname?: string; search?: string; hash?: string } } | null)?.from;
  const target = passwordChangeRequired ? "/settings/account" : from?.pathname?.startsWith("/") && !from.pathname.startsWith("//")
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
    <form className="flex flex-col gap-4 mt-2" onSubmit={login}>
      <div className="flex flex-col gap-1.5">
        <label className="text-xs font-semibold text-slate-700 flex items-center gap-1.5" htmlFor="username">
          <UserIcon className="h-3.5 w-3.5 text-slate-400" />
          <span>Nama Pengguna</span>
        </label>
        <Input
          id="username"
          name="username"
          autoComplete="username"
          placeholder="contoh: owner / operator"
          required
          maxLength={128}
          autoFocus
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <label className="text-xs font-semibold text-slate-700 flex items-center gap-1.5" htmlFor="password">
          <Lock className="h-3.5 w-3.5 text-slate-400" />
          <span>Kata Sandi</span>
        </label>
        <Input
          id="password"
          name="password"
          type="password"
          autoComplete="current-password"
          placeholder="••••••••"
          required
          maxLength={1024}
        />
      </div>

      {error && <ErrorNotice message={error.message} />}

      <Button type="submit" disabled={pending} className="w-full mt-1">
        {pending ? "Memeriksa kredensial…" : "Masuk ke Akun"}
      </Button>
    </form>
  );
}
