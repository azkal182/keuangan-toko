import { useCallback, useState, type FormEvent } from "react";
import { ApiFailure, api, failure, readResult, useResource, type Schema } from "./api";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Badge } from "./components/ui/badge";
import { Card, CardContent } from "./components/ui/card";
import { Dialog } from "./components/ui/dialog";
import { ErrorNotice, EmptyState } from "./components/presentation";

type User = Schema["User"];
type UserRole = Schema["UserRole"];
const roles: { value: UserRole; label: string }[] = [
  { value: "owner", label: "Pemilik" },
  { value: "operator", label: "Operator" },
  { value: "viewer", label: "Pembaca" },
];
const roleLabels: Record<UserRole, string> = { owner: "Pemilik", operator: "Operator", viewer: "Pembaca" };

export function UserManagementPage({ csrf, currentUserId }: { csrf: string; currentUserId: string }) {
  const [refresh, setRefresh] = useState(0);
  const [editing, setEditing] = useState<User | null>(null);
  const [creating, setCreating] = useState(false);
  const [pendingDeactivate, setPendingDeactivate] = useState<User | null>(null);
  const [resetting, setResetting] = useState<User | null>(null);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  const load = useCallback(async (signal: AbortSignal) =>
    readResult(await api.GET("/api/v1/users", { signal })), [refresh]);
  const { state, reload } = useResource(load);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const username = String(data.get("username") ?? "").trim();
    const password = String(data.get("password") ?? "");
    const role = String(data.get("role")) as UserRole;
    const is_active = data.get("is_active") === "on";
    if (!editing && password.length < 15) {
      setError(new ApiFailure("Kata sandi harus minimal 15 karakter."));
      return;
    }
    setPending(true);
    setError(undefined);
    try {
      if (editing) {
        readResult(await api.PATCH("/api/v1/users/{id}", {
          params: { path: { id: editing.id }, header: { "X-CSRF-Token": csrf } },
          body: { username, role, is_active },
        }));
      } else {
        readResult(await api.POST("/api/v1/users", {
          params: { header: { "X-CSRF-Token": csrf } },
          body: { username, role, password },
        }));
      }
      setEditing(null);
      setCreating(false);
      setNotice(editing ? "Perubahan pengguna tersimpan." : "Pengguna baru berhasil dibuat.");
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
    if (temporary_password.length < 15) { setError(new ApiFailure("Kata sandi sementara minimal 15 karakter.")); return; }
    setPending(true); setError(undefined);
    try {
      readResult(await api.POST("/api/v1/users/{id}/password-reset", {
        params: { path: { id: resetting.id }, header: { "X-CSRF-Token": csrf } },
        body: { temporary_password },
      }));
      setNotice(`Kata sandi ${resetting.username} direset. Pengguna wajib menggantinya saat masuk.`);
      setResetting(null);
      reload();
    } catch (cause) { setError(failure(cause)); }
    finally { setPending(false); }
  }

  async function deactivate(user: User) {
    setPending(true);
    setError(undefined);
    try {
      readResult(await api.DELETE("/api/v1/users/{id}", {
        params: { path: { id: user.id }, header: { "X-CSRF-Token": csrf } },
      }));
      setPendingDeactivate(null);
      setNotice(`Akun ${user.username} dinonaktifkan. Riwayat audit tetap tersimpan.`);
      reload();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  const users = state.status === "ready" ? state.data.data : [];
  return (
    <section className="space-y-5" aria-labelledby="users-title">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 id="users-title" className="text-lg font-semibold text-slate-900">Manajemen Pengguna</h2>
          <p className="mt-1 text-sm text-slate-500">Kelola akun individu dan hak akses. Akun yang dinonaktifkan tetap tercatat pada riwayat audit.</p>
        </div>
        <Button onClick={() => { setError(undefined); setCreating(true); }}>Tambah pengguna</Button>
      </div>
      {notice && <p role="status" className="rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-900">{notice}</p>}
      {error && <ErrorNotice message={error.message} retry={state.status === "error" ? reload : undefined} />}
      {state.status === "loading" && <p role="status" className="p-8 text-center text-sm text-slate-500">Memuat pengguna…</p>}
      {state.status === "error" && !error && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && users.length === 0 && <EmptyState title="Belum ada akun pengguna" description="Tambahkan akun untuk operator atau pembaca laporan." />}
      {state.status === "ready" && users.length > 0 && (
        <div className="grid gap-3">
          {users.map((user) => <Card key={user.id} className="border-slate-200 shadow-2xs">
            <CardContent className="flex flex-wrap items-center justify-between gap-4 p-4">
              <div className="flex min-w-0 items-center gap-3">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-emerald-50 text-sm font-semibold text-emerald-900">{user.username.slice(0, 1).toUpperCase()}</div>
                <div className="min-w-0">
                  <p className="truncate font-semibold text-slate-900">{user.username}{user.id === currentUserId && <span className="ml-2 text-xs font-normal text-slate-500">(akun Anda)</span>}</p>
                  <p className="mt-1 text-xs text-slate-500">Dibuat {new Date(user.created_at).toLocaleDateString("id-ID", { dateStyle: "medium" })}</p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Badge variant={user.role === "owner" ? "success" : "info"}>{roleLabels[user.role]}</Badge>
                <Badge variant={user.is_active ? "success" : "secondary"}>{user.is_active ? "Aktif" : "Nonaktif"}</Badge>
                <Button variant="outline" size="sm" onClick={() => { setError(undefined); setEditing(user); }}>Ubah</Button>
                {user.id !== currentUserId && user.is_active && <Button variant="outline" size="sm" onClick={() => { setError(undefined); setResetting(user); }}>Reset kata sandi</Button>}
                {user.is_active && user.id !== currentUserId && <Button variant="destructive" size="sm" onClick={() => { setError(undefined); setPendingDeactivate(user); }}>Nonaktifkan</Button>}
              </div>
            </CardContent>
          </Card>)}
        </div>
      )}

      <Dialog open={creating || !!editing} onOpenChange={(open) => { if (!open && !pending) { setCreating(false); setEditing(null); setError(undefined); } }} title={editing ? `Ubah pengguna ${editing.username}` : "Tambah pengguna"} description="Akun baru harus mengganti kata sandi sementara saat masuk pertama kali.">
        <form className="space-y-4" onSubmit={save}>
          {error && <ErrorNotice message={error.message} />}
          <label className="block space-y-1.5 text-sm font-medium text-slate-700">Nama pengguna
            <Input name="username" defaultValue={editing?.username} required maxLength={128} autoComplete="username" />
          </label>
          <label className="block space-y-1.5 text-sm font-medium text-slate-700">Role
            <select className="flex h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm" name="role" defaultValue={editing?.role ?? "operator"} disabled={editing?.id === currentUserId}>
              {roles.map((role) => <option key={role.value} value={role.value}>{role.label}</option>)}
            </select>
          </label>
          {editing?.id === currentUserId && <input type="hidden" name="role" value={editing.role} />}
          {editing && editing.id !== currentUserId && <label className="flex items-center gap-2 text-sm text-slate-700"><input type="checkbox" name="is_active" defaultChecked={editing.is_active} /> Akun aktif</label>}
          {editing?.id === currentUserId && <input type="hidden" name="is_active" value="on" />}
          {!editing && <label className="block space-y-1.5 text-sm font-medium text-slate-700">Kata sandi sementara
            <Input name="password" type="password" minLength={15} required maxLength={1024} autoComplete="new-password" />
          </label>}
          {editing?.id === currentUserId && <p className="text-xs text-slate-500">Role dan status akun Anda tidak dapat diubah dari halaman ini.</p>}
          <div className="flex justify-end gap-2 pt-2">
            <Button type="button" variant="outline" onClick={() => { setCreating(false); setEditing(null); setError(undefined); }} disabled={pending}>Batal</Button>
            <Button type="submit" disabled={pending}>{pending ? "Menyimpan…" : "Simpan"}</Button>
          </div>
        </form>
      </Dialog>

      <Dialog open={!!resetting} onOpenChange={(open) => { if (!open && !pending) { setResetting(null); setError(undefined); } }} title={`Reset kata sandi ${resetting?.username ?? ""}`} description="Pengguna akan diminta membuat kata sandi baru saat masuk berikutnya. Sesi aktifnya akan dicabut.">
        <form className="space-y-4" onSubmit={resetPassword}>
          {error && <ErrorNotice message={error.message} />}
          <label className="block space-y-1.5 text-sm font-medium text-slate-700">Kata sandi sementara
            <Input name="temporary_password" type="password" required minLength={15} maxLength={1024} autoComplete="new-password" />
          </label>
          <div className="flex justify-end gap-2"><Button type="button" variant="outline" onClick={() => setResetting(null)} disabled={pending}>Batal</Button><Button type="submit" disabled={pending}>{pending ? "Mereset…" : "Reset kata sandi"}</Button></div>
        </form>
      </Dialog>

      <Dialog open={!!pendingDeactivate} onOpenChange={(open) => { if (!open && !pending) setPendingDeactivate(null); }} title="Nonaktifkan akun?" description={`${pendingDeactivate?.username} tidak akan bisa masuk lagi. Riwayat transaksi dan audit tetap disimpan.`}>
        {error && <div className="mb-4"><ErrorNotice message={error.message} /></div>}
        <div className="flex justify-end gap-2">
          <Button variant="outline" disabled={pending} onClick={() => setPendingDeactivate(null)}>Batal</Button>
          <Button variant="destructive" disabled={pending || !pendingDeactivate} onClick={() => pendingDeactivate && void deactivate(pendingDeactivate)}>{pending ? "Menonaktifkan…" : "Ya, nonaktifkan"}</Button>
        </div>
      </Dialog>
    </section>
  );
}

export function ChangePasswordPage({ csrf, onChanged }: { csrf: string; onChanged?: () => void }) {
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState<ApiFailure>();

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const current_password = String(data.get("current_password") ?? "");
    const new_password = String(data.get("new_password") ?? "");
    const confirmation = String(data.get("confirmation") ?? "");
    if (new_password.length < 15) { setError(new ApiFailure("Kata sandi baru minimal 15 karakter.")); return; }
    if (new_password !== confirmation) { setError(new ApiFailure("Konfirmasi kata sandi tidak cocok.")); return; }
    setPending(true); setError(undefined); setNotice("");
    try {
      readResult(await api.POST("/api/v1/auth/password", {
        params: { header: { "X-CSRF-Token": csrf } },
        body: { current_password, new_password },
      }));
      form.reset();
      setNotice("Kata sandi berhasil diganti. Sesi aktif Anda tetap berjalan; sesi lain telah dikeluarkan.");
      onChanged?.();
    } catch (cause) { setError(failure(cause)); }
    finally { setPending(false); }
  }

  return <section className="mx-auto w-full max-w-2xl space-y-4" aria-labelledby="password-title">
    <div><h2 id="password-title" className="text-lg font-semibold text-slate-900">Ganti Kata Sandi</h2><p className="mt-1 text-sm text-slate-500">Gunakan kata sandi baru yang unik dan minimal 15 karakter.</p></div>
    <Card><CardContent className="p-5">
      <form className="space-y-4" onSubmit={submit}>
        {error && <ErrorNotice message={error.message} />}
        {notice && <p role="status" className="rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-900">{notice}</p>}
        <label className="block space-y-1.5 text-sm font-medium text-slate-700">Kata sandi saat ini<Input name="current_password" type="password" autoComplete="current-password" required maxLength={1024} /></label>
        <label className="block space-y-1.5 text-sm font-medium text-slate-700">Kata sandi baru<Input name="new_password" type="password" autoComplete="new-password" required minLength={15} maxLength={1024} /></label>
        <label className="block space-y-1.5 text-sm font-medium text-slate-700">Ulangi kata sandi baru<Input name="confirmation" type="password" autoComplete="new-password" required minLength={15} maxLength={1024} /></label>
        <div className="flex justify-end"><Button type="submit" disabled={pending}>{pending ? "Menyimpan…" : "Ganti kata sandi"}</Button></div>
      </form>
    </CardContent></Card>
  </section>;
}
