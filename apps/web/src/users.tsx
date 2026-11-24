import { useCallback, useState, type FormEvent } from "react";
import {
  Users,
  Shield,
  UserCheck,
  UserX,
  KeyRound,
  Plus,
  AlertTriangle,
  CheckCircle2,
  Lock,
  UserCog,
  RotateCcw,
} from "lucide-react";
import { ApiFailure, api, failure, readResult, useResource, type Schema } from "./api";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "./components/ui/card";
import { Dialog } from "./components/ui/dialog";
import { ErrorNotice, EmptyState } from "./components/presentation";

type User = Schema["User"];
type UserRole = Schema["UserRole"];

const roles: { value: UserRole; label: string; desc: string }[] = [
  { value: "owner", label: "Pemilik (Owner)", desc: "Akses penuh termasuk koreksi, alokasi, merge, dan kelola akun" },
  { value: "operator", label: "Operator Kasir", desc: "Dapat mencatat penjualan kredit, pembayaran, dan import staging" },
  { value: "viewer", label: "Pembaca (Viewer)", desc: "Hanya dapat melihat daftar piutang dan laporan" },
];

const roleLabels: Record<UserRole, { label: string; variant: "success" | "info" | "secondary" }> = {
  owner: { label: "Pemilik", variant: "success" },
  operator: { label: "Operator", variant: "info" },
  viewer: { label: "Pembaca", variant: "secondary" },
};

export function UserManagementPage({ csrf, currentUserId }: { csrf: string; currentUserId: string }) {
  const [refresh, setRefresh] = useState(0);
  const [editing, setEditing] = useState<User | null>(null);
  const [creating, setCreating] = useState(false);
  const [pendingDeactivate, setPendingDeactivate] = useState<User | null>(null);
  const [resetting, setResetting] = useState<User | null>(null);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);

  const load = useCallback(
    async (signal: AbortSignal) => readResult(await api.GET("/api/v1/users", { signal })),
    [refresh],
  );
  const { state, reload } = useResource(load);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const username = String(data.get("username") ?? "").trim();
    const password = String(data.get("password") ?? "");
    const role = String(data.get("role")) as UserRole;
    const is_active = data.get("is_active") === "on";

    if (!editing && password.length < 15) {
      setError(new ApiFailure("Kata sandi sementara harus minimal 15 karakter sesuai kebijakan keamanan sistem."));
      return;
    }

    setPending(true);
    setError(undefined);
    try {
      if (editing) {
        readResult(
          await api.PATCH("/api/v1/users/{id}", {
            params: { path: { id: editing.id }, header: { "X-CSRF-Token": csrf } },
            body: { username, role, is_active },
          }),
        );
      } else {
        readResult(
          await api.POST("/api/v1/users", {
            params: { header: { "X-CSRF-Token": csrf } },
            body: { username, role, password },
          }),
        );
      }
      setEditing(null);
      setCreating(false);
      setNotice(editing ? `Perubahan akun "${username}" berhasil disimpan.` : `Pengguna baru "${username}" berhasil ditambahkan.`);
      setRefresh((value) => value + 1);
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  async function resetPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!resetting) return;
    const temporary_password = String(new FormData(event.currentTarget).get("temporary_password") ?? "");
    if (temporary_password.length < 15) {
      setError(new ApiFailure("Kata sandi sementara harus minimal 15 karakter."));
      return;
    }
    setPending(true);
    setError(undefined);
    try {
      readResult(
        await api.POST("/api/v1/users/{id}/password-reset", {
          params: { path: { id: resetting.id }, header: { "X-CSRF-Token": csrf } },
          body: { temporary_password },
        }),
      );
      setNotice(`Kata sandi akun ${resetting.username} berhasil direset. Pengguna wajib menggantinya saat masuk pertama kali.`);
      setResetting(null);
      reload();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  async function deactivate(user: User) {
    setPending(true);
    setError(undefined);
    try {
      readResult(
        await api.DELETE("/api/v1/users/{id}", {
          params: { path: { id: user.id }, header: { "X-CSRF-Token": csrf } },
        }),
      );
      setPendingDeactivate(null);
      setNotice(`Akun ${user.username} telah dinonaktifkan. Seluruh riwayat transaksi dan audit tetap tersimpan.`);
      reload();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  const users = state.status === "ready" ? state.data.data : [];
  const totalOwners = users.filter((u) => u.role === "owner").length;
  const totalOperators = users.filter((u) => u.role === "operator").length;
  const totalActive = users.filter((u) => u.is_active).length;

  return (
    <div className="space-y-6">
      {/* KPI Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="border-slate-200/90 shadow-2xs">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Total Pengguna</span>
              <Users className="h-4 w-4 text-emerald-800" />
            </div>
            <CardTitle className="text-2xl font-bold font-mono text-slate-900 mt-1">
              {users.length} <span className="text-xs font-normal text-slate-500">akun terdaftar</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">Akun terdaftar dalam sistem</p>
          </CardContent>
        </Card>

        <Card className="border-slate-200/90 shadow-2xs bg-gradient-to-br from-white to-emerald-50/20">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Peran Pemilik (Owner)</span>
              <Shield className="h-4 w-4 text-emerald-700" />
            </div>
            <CardTitle className="text-2xl font-bold font-mono text-emerald-900 mt-1">
              {totalOwners} <span className="text-xs font-normal text-slate-500">akun</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">Hak akses administratif penuh</p>
          </CardContent>
        </Card>

        <Card className="border-slate-200/90 shadow-2xs">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Operator Kasir</span>
              <UserCog className="h-4 w-4 text-sky-700" />
            </div>
            <CardTitle className="text-2xl font-bold font-mono text-slate-900 mt-1">
              {totalOperators} <span className="text-xs font-normal text-slate-500">akun</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">Penjualan kredit & pencatatan bayar</p>
          </CardContent>
        </Card>

        <Card className="border-slate-200/90 shadow-2xs">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Status Keaktifan</span>
              <UserCheck className="h-4 w-4 text-emerald-600" />
            </div>
            <CardTitle className="text-2xl font-bold font-mono text-emerald-800 mt-1">
              {totalActive} <span className="text-xs font-normal text-slate-500">aktif</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">Akun yang memiliki izin login</p>
          </CardContent>
        </Card>
      </div>

      {/* Main Table Card */}
      <Card className="border-slate-200/90 shadow-2xs">
        <CardHeader className="border-b border-slate-100 pb-4">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <CardTitle className="text-base font-bold text-slate-900 flex items-center gap-2">
                <UserCog className="h-4 w-4 text-emerald-800" />
                <span>Daftar Akun Pengguna</span>
              </CardTitle>
              <p className="text-xs text-slate-500 mt-0.5">
                Kelola kredensial, hak otorisasi peran, dan reset sandi operator secara aman.
              </p>
            </div>

            <Button
              onClick={() => {
                setError(undefined);
                setCreating(true);
              }}
              size="sm"
              className="gap-1.5 text-xs font-semibold shadow-sm"
            >
              <Plus className="h-4 w-4" />
              <span>Tambah Pengguna</span>
            </Button>
          </div>
        </CardHeader>

        <CardContent className="p-0">
          {notice && (
            <div className="m-4 flex items-center gap-2 p-3.5 rounded-xl border border-emerald-200 bg-emerald-50 text-emerald-900 text-xs font-medium">
              <CheckCircle2 className="h-4 w-4 text-emerald-600 shrink-0" />
              <span>{notice}</span>
            </div>
          )}

          {error && (
            <div className="m-4">
              <ErrorNotice message={error.message} retry={state.status === "error" ? reload : undefined} />
            </div>
          )}

          {state.status === "loading" && (
            <div className="p-12 text-center text-sm text-slate-500">
              <div className="flex items-center justify-center gap-3">
                <div className="h-5 w-5 border-2 border-emerald-700 border-t-transparent rounded-full animate-spin" />
                <span>Memuat data pengguna…</span>
              </div>
            </div>
          )}

          {state.status === "error" && !error && (
            <div className="p-6">
              <ErrorNotice message={state.error.message} retry={reload} />
            </div>
          )}

          {state.status === "ready" && users.length === 0 && (
            <EmptyState
              title="Belum ada akun pengguna"
              description="Tambahkan akun untuk operator kasir atau pembaca laporan keuangan toko."
            />
          )}

          {state.status === "ready" && users.length > 0 && (
            <div className="relative w-full overflow-x-auto">
              <table className="w-full text-left text-xs border-collapse">
                <thead>
                  <tr className="border-b border-slate-200 bg-slate-50 text-slate-600 font-semibold uppercase text-[11px]">
                    <th className="p-3 pl-6">Pengguna</th>
                    <th className="p-3">Peran (Role)</th>
                    <th className="p-3">Status</th>
                    <th className="p-3">Tanggal Dibuat</th>
                    <th className="p-3 pr-6 text-right">Aksi</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 bg-white">
                  {users.map((user) => {
                    const isSelf = user.id === currentUserId;
                    const roleInfo = roleLabels[user.role] ?? { label: user.role, variant: "secondary" as const };

                    return (
                      <tr key={user.id} className="hover:bg-slate-50/70 transition-colors">
                        <td className="p-3 pl-6">
                          <div className="flex items-center gap-3">
                            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-emerald-800 to-emerald-950 text-xs font-bold text-white shadow-2xs">
                              {user.username.slice(0, 1).toUpperCase()}
                            </div>
                            <div>
                              <div className="flex items-center gap-2">
                                <span className="font-semibold text-slate-900 text-sm">{user.username}</span>
                                {isSelf && (
                                  <span className="text-[10px] font-medium px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
                                    Akun Anda
                                  </span>
                                )}
                              </div>
                              <span className="text-[11px] text-slate-400 font-mono">ID: {user.id.slice(0, 8)}…</span>
                            </div>
                          </div>
                        </td>

                        <td className="p-3">
                          <Badge variant={roleInfo.variant} className="text-[11px] font-medium">
                            {roleInfo.label}
                          </Badge>
                        </td>

                        <td className="p-3">
                          <Badge variant={user.is_active ? "success" : "secondary"} className="text-[11px]">
                            {user.is_active ? "Aktif" : "Nonaktif"}
                          </Badge>
                        </td>

                        <td className="p-3 font-mono text-slate-600">
                          {new Date(user.created_at).toLocaleDateString("id-ID", {
                            day: "numeric",
                            month: "short",
                            year: "numeric",
                          })}
                        </td>

                        <td className="p-3 pr-6 text-right">
                          <div className="flex items-center justify-end gap-1.5">
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => {
                                setError(undefined);
                                setEditing(user);
                              }}
                              className="text-xs h-8"
                            >
                              Ubah
                            </Button>

                            {!isSelf && user.is_active && (
                              <Button
                                variant="outline"
                                size="sm"
                                onClick={() => {
                                  setError(undefined);
                                  setResetting(user);
                                }}
                                className="text-xs h-8 gap-1 text-amber-800 border-amber-200 hover:bg-amber-50"
                              >
                                <KeyRound className="h-3 w-3" />
                                <span>Reset Sandi</span>
                              </Button>
                            )}

                            {!isSelf && user.is_active && (
                              <Button
                                variant="destructive"
                                size="sm"
                                onClick={() => {
                                  setError(undefined);
                                  setPendingDeactivate(user);
                                }}
                                className="text-xs h-8"
                              >
                                Nonaktifkan
                              </Button>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create / Edit Dialog */}
      <Dialog
        open={creating || !!editing}
        onOpenChange={(open) => {
          if (!open && !pending) {
            setCreating(false);
            setEditing(null);
            setError(undefined);
          }
        }}
        title={editing ? `Ubah Data Akun: ${editing.username}` : "Tambah Pengguna Baru"}
        description={
          editing
            ? "Perbarui nama pengguna, peran otorisasi, atau status keaktifan akun."
            : "Akun baru wajib mengganti kata sandi sementara saat pertama kali masuk ke sistem."
        }
      >
        <form className="space-y-4" onSubmit={save}>
          {error && <ErrorNotice message={error.message} />}

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="user-username">
              Nama Pengguna (Username) <span className="text-rose-600">*</span>
            </label>
            <Input
              id="user-username"
              name="username"
              defaultValue={editing?.username}
              required
              maxLength={128}
              placeholder="contoh: kasir_pagi"
              autoComplete="username"
              className="text-xs"
            />
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="user-role">
              Peran Hak Akses (Role)
            </label>
            <select
              id="user-role"
              name="role"
              defaultValue={editing?.role ?? "operator"}
              disabled={editing?.id === currentUserId}
              className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
            >
              {roles.map((r) => (
                <option key={r.value} value={r.value}>
                  {r.label} — {r.desc}
                </option>
              ))}
            </select>
            {editing?.id === currentUserId && <input type="hidden" name="role" value={editing.role} />}
          </div>

          {!editing && (
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="user-password">
                Kata Sandi Sementara <span className="text-rose-600">*</span>
              </label>
              <Input
                id="user-password"
                name="password"
                type="password"
                minLength={15}
                required
                maxLength={1024}
                placeholder="Minimal 15 karakter…"
                autoComplete="new-password"
                className="text-xs"
              />
              <p className="text-[11px] text-slate-500">
                Sesuai kebijakan keamanan, kata sandi minimal 15 karakter. Pengguna akan diminta mengubahnya saat masuk.
              </p>
            </div>
          )}

          {editing && editing.id !== currentUserId && (
            <label className="flex items-center gap-2 cursor-pointer pt-1">
              <input
                type="checkbox"
                name="is_active"
                defaultChecked={editing.is_active}
                className="h-4 w-4 rounded border-slate-300 text-emerald-800 focus:ring-emerald-700"
              />
              <span className="text-xs font-medium text-slate-700">Akun Aktif (Dapat Masuk ke Sistem)</span>
            </label>
          )}

          {editing?.id === currentUserId && (
            <div className="p-3 bg-slate-50 rounded-lg text-xs text-slate-500 border border-slate-200">
              Role dan status akun Anda tidak dapat diubah sendiri dari panel ini untuk mencegah hilangnya akses pemilik.
            </div>
          )}

          <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-200">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                setCreating(false);
                setEditing(null);
                setError(undefined);
              }}
              disabled={pending}
            >
              Batal
            </Button>
            <Button type="submit" size="sm" disabled={pending} className="font-semibold">
              {pending ? "Menyimpan…" : "Simpan Data"}
            </Button>
          </div>
        </form>
      </Dialog>

      {/* Reset Password Dialog */}
      <Dialog
        open={!!resetting}
        onOpenChange={(open) => {
          if (!open && !pending) {
            setResetting(null);
            setError(undefined);
          }
        }}
        title={`Reset Kata Sandi: ${resetting?.username ?? ""}`}
        description="Buat kata sandi sementara baru. Seluruh sesi aktif pengguna ini akan langsung dicabut dan pengguna wajib mengganti sandi saat masuk."
      >
        <form className="space-y-4" onSubmit={resetPassword}>
          {error && <ErrorNotice message={error.message} />}

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="reset-pass">
              Kata Sandi Sementara Baru <span className="text-rose-600">*</span>
            </label>
            <Input
              id="reset-pass"
              name="temporary_password"
              type="password"
              required
              minLength={15}
              maxLength={1024}
              placeholder="Minimal 15 karakter…"
              autoComplete="new-password"
              className="text-xs"
            />
            <p className="text-[11px] text-slate-500">Minimal 15 karakter.</p>
          </div>

          <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-200">
            <Button type="button" variant="outline" size="sm" onClick={() => setResetting(null)} disabled={pending}>
              Batal
            </Button>
            <Button type="submit" size="sm" disabled={pending} className="font-semibold">
              {pending ? "Mereset…" : "Konfirmasi Reset Sandi"}
            </Button>
          </div>
        </form>
      </Dialog>

      {/* Deactivate Dialog */}
      <Dialog
        open={!!pendingDeactivate}
        onOpenChange={(open) => {
          if (!open && !pending) setPendingDeactivate(null);
        }}
        title="Konfirmasi Penonaktifan Akun"
      >
        <div className="space-y-4">
          <div className="flex items-start gap-3 p-4 rounded-xl border border-rose-200 bg-rose-50 text-rose-900 text-xs">
            <AlertTriangle className="h-5 w-5 text-rose-600 shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">
                Apakah Anda yakin ingin menonaktifkan akun "{pendingDeactivate?.username}"?
              </p>
              <p className="mt-1 text-rose-800">
                Pengguna ini tidak akan bisa masuk lagi ke sistem. Seluruh data transaksi penjualan kredit, pembayaran, dan riwayat audit yang pernah dicatat oleh akun ini tetap tersimpan utuh di basis data.
              </p>
            </div>
          </div>

          {error && <ErrorNotice message={error.message} />}

          <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-200">
            <Button variant="outline" size="sm" disabled={pending} onClick={() => setPendingDeactivate(null)}>
              Batal
            </Button>
            <Button
              variant="destructive"
              size="sm"
              disabled={pending || !pendingDeactivate}
              onClick={() => pendingDeactivate && void deactivate(pendingDeactivate)}
            >
              {pending ? "Menonaktifkan…" : "Ya, Nonaktifkan Akun"}
            </Button>
          </div>
        </div>
      </Dialog>
    </div>
  );
}

export function ChangePasswordPage({ csrf, onChanged }: { csrf: string; onChanged?: () => void }) {
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState<ApiFailure>();
  const [newPass, setNewPass] = useState("");
  const [confirmPass, setConfirmPass] = useState("");

  const isLongEnough = newPass.length >= 15;
  const isMatching = newPass.length > 0 && newPass === confirmPass;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const current_password = String(data.get("current_password") ?? "");
    const new_password = String(data.get("new_password") ?? "");
    const confirmation = String(data.get("confirmation") ?? "");

    if (new_password.length < 15) {
      setError(new ApiFailure("Kata sandi baru harus minimal 15 karakter."));
      return;
    }
    if (new_password !== confirmation) {
      setError(new ApiFailure("Konfirmasi kata sandi tidak cocok."));
      return;
    }

    setPending(true);
    setError(undefined);
    setNotice("");
    try {
      readResult(
        await api.POST("/api/v1/auth/password", {
          params: { header: { "X-CSRF-Token": csrf } },
          body: { current_password, new_password },
        }),
      );
      form.reset();
      setNewPass("");
      setConfirmPass("");
      setNotice("Kata sandi berhasil diganti. Sesi Anda tetap aktif, dan seluruh sesi lain di perangkat lain telah dicabut.");
      onChanged?.();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <Card className="border-slate-200/90 shadow-2xs overflow-hidden">
        <div className="bg-gradient-to-r from-slate-900 to-slate-800 p-6 text-white flex items-center gap-4">
          <div className="flex items-center justify-center h-12 w-12 rounded-2xl bg-white/10 ring-1 ring-white/20">
            <Lock className="h-6 w-6 text-white" />
          </div>
          <div>
            <h2 className="text-lg font-bold">Keamanan Akun & Kata Sandi</h2>
            <p className="text-xs text-slate-300 mt-0.5">
              Gunakan kata sandi yang kuat dan unik untuk melindungi catatan keuangan toko.
            </p>
          </div>
        </div>

        <CardContent className="p-6">
          <form className="space-y-4" onSubmit={submit}>
            {error && <ErrorNotice message={error.message} />}

            {notice && (
              <div className="flex items-center gap-2 p-3.5 rounded-xl border border-emerald-200 bg-emerald-50 text-emerald-900 text-xs font-medium">
                <CheckCircle2 className="h-4 w-4 text-emerald-600 shrink-0" />
                <span>{notice}</span>
              </div>
            )}

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="curr-pass">
                Kata Sandi Saat Ini <span className="text-rose-600">*</span>
              </label>
              <Input
                id="curr-pass"
                name="current_password"
                type="password"
                autoComplete="current-password"
                required
                maxLength={1024}
                placeholder="Masukkan kata sandi lama Anda…"
                className="text-xs"
              />
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="new-pass">
                Kata Sandi Baru <span className="text-rose-600">*</span>
              </label>
              <Input
                id="new-pass"
                name="new_password"
                type="password"
                autoComplete="new-password"
                required
                minLength={15}
                maxLength={1024}
                value={newPass}
                onChange={(e) => setNewPass(e.target.value)}
                placeholder="Minimal 15 karakter…"
                className="text-xs"
              />
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="confirm-pass">
                Ulangi Kata Sandi Baru <span className="text-rose-600">*</span>
              </label>
              <Input
                id="confirm-pass"
                name="confirmation"
                type="password"
                autoComplete="new-password"
                required
                minLength={15}
                maxLength={1024}
                value={confirmPass}
                onChange={(e) => setConfirmPass(e.target.value)}
                placeholder="Ketik ulang kata sandi baru…"
                className="text-xs"
              />
            </div>

            {/* Live requirement badges */}
            <div className="p-3 rounded-xl bg-slate-50 border border-slate-200 text-xs space-y-1.5">
              <span className="font-semibold text-slate-600 block text-[11px] uppercase tracking-wider">
                Syarat Keamanan:
              </span>
              <div className="flex items-center gap-2">
                <div
                  className={`h-2 w-2 rounded-full ${
                    isLongEnough ? "bg-emerald-600" : "bg-slate-300"
                  }`}
                />
                <span className={isLongEnough ? "text-emerald-900 font-medium" : "text-slate-500"}>
                  Minimal 15 karakter {newPass.length > 0 ? `(${newPass.length}/15)` : ""}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <div
                  className={`h-2 w-2 rounded-full ${
                    isMatching ? "bg-emerald-600" : "bg-slate-300"
                  }`}
                />
                <span className={isMatching ? "text-emerald-900 font-medium" : "text-slate-500"}>
                  Konfirmasi kata sandi cocok
                </span>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-200">
              <Button type="submit" disabled={pending} className="gap-1.5 font-semibold text-xs">
                <KeyRound className="h-3.5 w-3.5" />
                <span>{pending ? "Menyimpan…" : "Simpan Kata Sandi Baru"}</span>
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
