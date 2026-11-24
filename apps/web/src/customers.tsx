import { lazy, Suspense, useCallback, useEffect, useState, type FormEvent } from "react";
import { Navigate, NavLink, Route, Routes, useLocation, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { api, failure, readResult, useResource, type ApiFailure, type Schema } from "./api";
import { Brand, ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { formatRupiah } from "./money";
import { Dialog } from "./components/ui/dialog";

const SalesPanel = lazy(() => import("./sales").then((module) => ({ default: module.SalesPanel })));
const PaymentsPanel = lazy(() => import("./payments").then((module) => ({ default: module.PaymentsPanel })));
const Ledger = lazy(() => import("./ledger").then((module) => ({ default: module.Ledger })));
const ReceivablesAging = lazy(() => import("./reports").then((module) => ({ default: module.ReceivablesAging })));
const ImportWorkspace = lazy(() => import("./imports").then((module) => ({ default: module.ImportWorkspace })));

const labels: Record<Schema["UserRole"], string> = { owner: "Pemilik", operator: "Operator", viewer: "Pembaca" };
const agingLabels = ["0–15 hari", "16–30 hari", "31–45 hari", "46–60 hari", "Lebih dari 60 hari", "Tanggal perlu ditinjau"];

type CustomerSummary = Schema["CustomerList"]["items"][number];
function unpackCustomer(value: CustomerSummary | Schema["Customer"]) {
  if ("customer" in value) return {
    customer: value.customer,
    outstanding: value.outstanding_idr,
    aging: [value.days_0_15_idr, value.days_16_30_idr, value.days_31_45_idr, value.days_46_60_idr, value.days_over_60_idr],
    exceptions: BigInt(value.unknown_date_idr) + BigInt(value.future_date_idr),
  };
  // Older API processes return Customer directly. Keep the UI alive during a rolling restart.
  return { customer: value, outstanding: undefined, aging: undefined, exceptions: undefined };
}

export function CustomerWorkspace({ session, onLogout }: {
  session: Schema["SessionDetails"];
  onLogout: () => void;
}) {
  const location = useLocation();
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState("");
  const [logoutError, setLogoutError] = useState<string>();
  const [loggingOut, setLoggingOut] = useState(false);
  const sessionHeader = { "X-CSRF-Token": session.csrf_token };
  const canWrite = session.user.role !== "viewer";

  useEffect(() => setCreating(false), [location.pathname]);

  function select(id?: string) {
    const from = `${location.pathname}${location.search}`;
    navigate(id ? `/customers/${id}` : "/customers", id ? { state: { from } } : undefined);
    setCreating(false);
    setNotice("");
  }

  async function logout() {
    setLoggingOut(true);
    setLogoutError(undefined);
    try {
      const result = await api.POST("/api/v1/auth/logout", { params: { header: sessionHeader } });
      if (result.response.status !== 401) readResult(result);
      onLogout();
    } catch (error) {
      setLogoutError(failure(error).message);
    } finally {
      setLoggingOut(false);
    }
  }

  return (
    <>
      <header className="topbar">
        <Brand />
        <div className="account">
          <div><strong>{session.user.username}</strong><span>{labels[session.user.role]}</span></div>
          <Button variant="outline" onClick={logout} disabled={loggingOut}>
            {loggingOut ? "Keluar…" : "Keluar"}
          </Button>
        </div>
      </header>
      <main className="workspace">
        <div className="workspace-heading">
          <h1>{location.pathname.startsWith("/reports/") ? "Laporan piutang" : location.pathname.startsWith("/imports") ? "Import workbook" : location.pathname.startsWith("/customers/") ? "Detail pelanggan" : "Pelanggan"}</h1>
          <nav className="workspace-nav" aria-label="Navigasi utama">
            <NavLink to="/customers" className={({ isActive }) => `button button-outline${isActive ? " nav-link-active" : ""}`}>Pelanggan</NavLink>
            <NavLink to="/reports/aging" className={({ isActive }) => `button button-outline${isActive ? " nav-link-active" : ""}`}>Umur piutang</NavLink>
            {canWrite && <NavLink to="/imports" className={({ isActive }) => `button button-outline${isActive ? " nav-link-active" : ""}`}>Import</NavLink>}
            {location.pathname === "/customers" && canWrite && <Button onClick={() => { setCreating(true); setNotice(""); }}>Tambah pelanggan</Button>}
          </nav>
        </div>
        {logoutError && <ErrorNotice message={logoutError} />}
        {notice && <p className="notice-success" role="status">{notice}</p>}
        <Suspense fallback={<p className="panel" role="status">Memuat halaman…</p>}>
          <Routes>
            <Route path="customers" element={<CustomerList onSelect={select} revision={revision} />} />
            <Route path="customers/:customerId" element={<CustomerDetailRoute csrf={session.csrf_token} canWrite={canWrite} canMerge={session.user.role === "owner"} revision={revision} onChanged={() => setRevision((value) => value + 1)} onNotice={setNotice} />} />
            <Route path="customers/:customerId/sales/:saleId" element={<CustomerDetailRoute csrf={session.csrf_token} canWrite={canWrite} canMerge={session.user.role === "owner"} revision={revision} onChanged={() => setRevision((value) => value + 1)} onNotice={setNotice} />} />
            <Route path="reports/aging" element={<ReceivablesAging />} />
            <Route path="imports" element={canWrite ? <ImportWorkspace csrf={session.csrf_token} owner={session.user.role === "owner"} /> : <Navigate to="/customers" replace />} />
            <Route path="imports/:batchId" element={canWrite ? <ImportWorkspace csrf={session.csrf_token} owner={session.user.role === "owner"} /> : <Navigate to="/customers" replace />} />
            <Route path="" element={<LegacyCustomerRedirect />} />
            <Route path="*" element={<Navigate to="/customers" replace />} />
          </Routes>
        </Suspense>
        <Dialog open={creating} onOpenChange={setCreating} title="Tambah pelanggan">
          <CustomerForm csrf={session.csrf_token} onCancel={() => setCreating(false)} onSaved={(customer) => {
            select(customer.id);
            setCreating(false);
            setRevision((value) => value + 1);
            setNotice("Pelanggan berhasil disimpan.");
          }} />
        </Dialog>
      </main>
    </>
  );
}

function LegacyCustomerRedirect() {
  const [searchParams] = useSearchParams();
  const customerId = searchParams.get("pelanggan");
  const saleId = searchParams.get("penjualan");
  if (!customerId) return <Navigate to="/customers" replace />;
  return <Navigate to={saleId ? `/customers/${customerId}/sales/${saleId}` : `/customers/${customerId}`} replace />;
}

function CustomerDetailRoute({ csrf, canWrite, canMerge, revision, onChanged, onNotice }: {
  csrf: string; canWrite: boolean; canMerge: boolean; revision: number;
  onChanged: () => void; onNotice: (message: string) => void;
}) {
  const { customerId, saleId } = useParams();
  const navigate = useNavigate();
  const location = useLocation();
  if (!customerId) return <Navigate to="/customers" replace />;
  const returnPath = (location.state as { from?: unknown } | null)?.from;
  const backToList = typeof returnPath === "string" && returnPath.startsWith("/customers") && !returnPath.startsWith("//")
    ? returnPath
    : "/customers";
  return <div className="customer-detail-page stack">
    <Button variant="outline" className="back-to-customers" onClick={() => navigate(backToList)}>← Ringkasan pelanggan</Button>
    <CustomerDetail
      key={`${customerId}:${revision}`}
      id={customerId}
      csrf={csrf}
      canWrite={canWrite}
      canMerge={canMerge}
      saleId={saleId}
      openSale={(id) => navigate(id ? `/customers/${customerId}/sales/${id}` : `/customers/${customerId}`, { state: location.state })}
      onDataChanged={onChanged}
      onSaved={() => { onChanged(); onNotice("Perubahan pelanggan berhasil disimpan."); }}
      onMerged={(id) => { navigate(`/customers/${id}`, { state: location.state }); onChanged(); onNotice("Pelanggan berhasil digabung."); }}
    />
  </div>;
}

function CustomerList({ onSelect, revision }: {
  onSelect: (id: string) => void;
  revision: number;
}) {
  const [searchParams, setSearchParams] = useSearchParams();
  const appliedSearch = searchParams.get("search") ?? "";
  const pageCursors = searchParams.getAll("page_cursor");
  const cursor = pageCursors.at(-1);
  const [search, setSearch] = useState(appliedSearch);
  useEffect(() => setSearch(appliedSearch), [appliedSearch]);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers", {
    params: { query: { search: appliedSearch, cursor } }, signal,
  })), [appliedSearch, cursor, revision]);
  const { state, reload } = useResource(load);
  function applySearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const next = new URLSearchParams(searchParams);
    const term = search.trim();
    if (term) next.set("search", term);
    else next.delete("search");
    next.delete("page_cursor");
    setSearchParams(next);
  }
  function movePage(direction: "previous" | "next", nextCursor?: string) {
    const cursors = direction === "previous" ? pageCursors.slice(0, -1) : nextCursor ? [...pageCursors, nextCursor] : pageCursors;
    const next = new URLSearchParams(searchParams);
    next.delete("page_cursor");
    for (const value of cursors) next.append("page_cursor", value);
    setSearchParams(next);
  }
  return (
    <section className="panel customer-list stack" aria-label="Daftar pelanggan">
      <div><h2>Ringkasan pelanggan</h2><p className="muted">Jumlah adalah total piutang terbuka. Bucket memakai umur kalender Asia/Jakarta; nominal bertanggal tidak diketahui atau masa depan ditampilkan di kolom peninjauan.</p></div>
      <form className="search-form" onSubmit={applySearch}>
        <Input aria-label="Nama atau kode pelanggan" placeholder="Nama atau kode" type="search" value={search} maxLength={256}
          onChange={(event) => setSearch(event.target.value)} />
        <Button type="submit" variant="outline">Cari</Button>
      </form>
      {state.status === "loading" && <p role="status">Memuat pelanggan…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && (
        <>
          {state.data.data.items.length === 0 ? <p className="muted">Tidak ada pelanggan yang ditemukan.</p> : (
            <>
              <div className="table-scroll customer-summary-desktop" tabIndex={0} role="region" aria-label="Ringkasan saldo pelanggan">
                <table className="customer-summary-table"><thead><tr><th scope="col">Pelanggan</th><th scope="col" className="number">Jumlah</th><th scope="col" className="number">0–15 Hari</th><th scope="col" className="number">16–30 Hari</th><th scope="col" className="number">31–45 Hari</th><th scope="col" className="number">46–60 Hari</th><th scope="col" className="number">Lebih dari 60 Hari</th><th scope="col" className="number">Tanggal perlu ditinjau</th></tr></thead>
                  <tbody>{state.data.data.items.map((item) => {
                    const { customer, outstanding, aging, exceptions } = unpackCustomer(item as CustomerSummary | Schema["Customer"]);
                    return <tr key={customer.id}>
                      <td><button type="button" className="customer-summary-link" onClick={() => onSelect(customer.id)}><strong>{customer.name}</strong><span className="table-note">{customer.code ?? "Tanpa kode"} {!customer.is_active && " · Nonaktif"}</span></button></td>
                      <td className="number">{outstanding === undefined ? "—" : formatRupiah(outstanding)}</td>{aging?.map((amount, index) => <td className="number" key={index}>{formatRupiah(amount)}</td>)}{aging && <td className="number" title="Jumlah piutang dengan tanggal tidak diketahui atau tanggal masa depan">{formatRupiah(exceptions!.toString())}</td>}
                    </tr>;
                  })}</tbody>
                </table>
              </div>
              <div className="customer-summary-cards" aria-label="Ringkasan pelanggan">
                {state.data.data.items.map((item) => <CustomerSummaryCard key={("customer" in item ? item.customer : item).id} item={item as CustomerSummary | Schema["Customer"]} onSelect={onSelect} />)}
              </div>
            </>
          )}
          {(pageCursors.length > 0 || state.data.meta.next_cursor) && (
            <div className="pagination">
              <Button variant="outline" disabled={!pageCursors.length} onClick={() => movePage("previous")}>Sebelumnya</Button>
              <Button variant="outline" disabled={!state.data.meta.next_cursor} onClick={() => movePage("next", state.data.meta.next_cursor ?? undefined)}>Berikutnya</Button>
            </div>
          )}
        </>
      )}
    </section>
  );
}

function CustomerSummaryCard({ item, onSelect }: { item: CustomerSummary | Schema["Customer"]; onSelect: (id: string) => void }) {
  const { customer, outstanding, aging, exceptions } = unpackCustomer(item);
  const amounts = [...(aging ?? []), exceptions?.toString()];
  return <article className="customer-summary-card">
    <div className="customer-summary-card-heading">
      <button type="button" className="customer-summary-link" onClick={() => onSelect(customer.id)}><strong>{customer.name}</strong><span className="table-note">{customer.code ?? "Tanpa kode"} {!customer.is_active && " · Nonaktif"}</span></button>
      <span><span className="table-note">Jumlah</span><strong>{outstanding === undefined ? "—" : formatRupiah(outstanding)}</strong></span>
    </div>
    <dl className="customer-aging-cards">
      {agingLabels.map((label, index) => <div key={label}><dt>{label}</dt><dd>{amounts[index] === undefined ? "—" : formatRupiah(amounts[index]!)}</dd></div>)}
    </dl>
  </article>;
}

function CustomerDetail({ id, csrf, canWrite, canMerge, saleId, openSale, onSaved, onDataChanged, onMerged }: {
  id: string; csrf: string; canWrite: boolean; canMerge: boolean;
  saleId?: string; openSale: (id?: string) => void;
  onSaved: () => void; onDataChanged: () => void; onMerged: (id: string) => void;
}) {
  const [mode, setMode] = useState<"view" | "edit" | "merge">("view");
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers/{id}", {
    params: { path: { id } }, signal,
  })).data, [id]);
  const { state, reload } = useResource(load);

  if (state.status === "loading") return <p className="panel" role="status">Memuat detail pelanggan…</p>;
  if (state.status === "error") return <ErrorNotice message={state.error.message} retry={reload} />;
  const { customer, outstanding_idr, customer_credit_idr } = state.data;
  return (
    <>
      <section className="panel stack">
        <div className="detail-heading">
          <div>
            <div className="detail-title">
              <h2>{customer.name}</h2>
              <span className={`badge ${customer.is_active ? "active" : "inactive"}`}>{customer.is_active ? "Aktif" : "Nonaktif"}</span>
            </div>
            <p className="muted">{customer.code ?? "Tanpa kode"}<span className="px-2">·</span>{customer.address ?? "Alamat belum diisi"}</p>
          </div>
          {mode === "view" && (canWrite || canMerge) && (
            <div className="actions">
              {canWrite && <Button variant="outline" onClick={() => setMode("edit")}>Ubah</Button>}
              {canMerge && <Button variant="plain" onClick={() => setMode("merge")}>Gabungkan</Button>}
            </div>
          )}
        </div>
        {mode === "merge" && <MergeForm customer={customer} csrf={csrf} onCancel={() => setMode("view")} onMerged={onMerged} />}
        <Dialog open={mode === "edit"} onOpenChange={(open) => { if (!open) setMode("view"); }} title="Ubah pelanggan">
          {mode === "edit" && <CustomerForm key={customer.version} customer={customer} csrf={csrf} onCancel={() => setMode("view")}
          onReload={reload} onSaved={() => { setMode("view"); reload(); onSaved(); }} />}
        </Dialog>
        <dl className="totals">
          <div><dt>Piutang terutang</dt><dd>{formatRupiah(outstanding_idr)}</dd></div>
          <div><dt>Saldo kredit pelanggan</dt><dd>{formatRupiah(customer_credit_idr)}</dd></div>
        </dl>
      </section>
      <SalesPanel customer={customer} csrf={csrf} canWrite={canWrite} owner={canMerge}
        saleId={saleId} onOpen={openSale} onPosted={reload} />
      <PaymentsPanel customer={customer} csrf={csrf} canWrite={canWrite} onChanged={onDataChanged} />
      <Ledger id={id} openSale={openSale} />
    </>
  );
}

function CustomerForm({ customer, csrf, onSaved, onCancel, onReload }: {
  customer?: Schema["Customer"]; csrf: string; onSaved: (customer: Schema["Customer"]) => void;
  onCancel: () => void; onReload?: () => void;
}) {
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const identity = {
      name: String(data.get("name")),
      code: String(data.get("code")) || null,
      address: String(data.get("address")) || null,
    };
    setError(undefined);
    setPending(true);
    try {
      const header = { "X-CSRF-Token": csrf };
      const result = customer ? await api.PATCH("/api/v1/customers/{id}", {
        params: { path: { id: customer.id }, header },
        body: { ...identity, expected_version: customer.version, is_active: data.get("is_active") === "on" },
      }) : await api.POST("/api/v1/customers", { params: { header }, body: identity });
      onSaved(readResult(result).data);
    } catch (error) {
      setError(failure(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <form className="stack identity-form" onSubmit={save}>
      <label>Nama pelanggan<Input name="name" defaultValue={customer?.name} maxLength={256} required aria-invalid={!!error?.fields.name} />
        {error?.fields.name && <span className="field-error">{error.fields.name}</span>}
      </label>
      <label>Kode pelanggan (opsional)<Input name="code" defaultValue={customer?.code ?? ""} maxLength={128} aria-invalid={!!error?.fields.code} />
        {error?.fields.code && <span className="field-error">{error.fields.code}</span>}
      </label>
      <label>Alamat (opsional)<textarea className="input" name="address" defaultValue={customer?.address ?? ""} rows={2} maxLength={2000} aria-invalid={!!error?.fields.address} />
        {error?.fields.address && <span className="field-error">{error.fields.address}</span>}
      </label>
      {customer && <label className="checkbox"><input type="checkbox" name="is_active" defaultChecked={customer.is_active} /> Pelanggan aktif</label>}
      {error && <ErrorNotice message={error.message} />}
      <div className="actions">
        {error?.fields.expected_version && onReload && <Button variant="outline" onClick={() => { setError(undefined); onReload(); }}>Muat ulang detail</Button>}
        <Button type="submit" disabled={pending}>{pending ? "Menyimpan…" : "Simpan"}</Button>
        <Button variant="outline" onClick={onCancel} disabled={pending}>Batal</Button>
      </div>
    </form>
  );
}

function MergeForm({ customer, csrf, onMerged, onCancel }: {
  customer: Schema["Customer"]; csrf: string; onMerged: (id: string) => void; onCancel: () => void;
}) {
  const [search, setSearch] = useState("");
  const [term, setTerm] = useState("");
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers", {
    params: { query: { search: term, limit: 100 } }, signal,
  })), [term]);
  const { state, reload } = useResource(load);
  const candidates = state.status === "ready" ? state.data.data.items.map((item) => unpackCustomer(item as CustomerSummary | Schema["Customer"]).customer).filter((item) => item.id !== customer.id) : [];

  async function merge(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const target_customer_id = String(data.get("target"));
    setError(undefined);
    setPending(true);
    try {
      readResult(await api.POST("/api/v1/customers/{id}/merge", {
        params: { path: { id: customer.id }, header: { "X-CSRF-Token": csrf } },
        body: { target_customer_id, reason: String(data.get("reason")), expected_version: customer.version },
      }));
      onMerged(target_customer_id);
    } catch (error) {
      setError(failure(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="stack identity-form">
      <h3>Gabungkan pelanggan</h3>
      <p className="muted">Riwayat akan dipindahkan ke pelanggan tujuan. Pelanggan ini akan dinonaktifkan.</p>
      <form className="search-form" onSubmit={(event) => { event.preventDefault(); setTerm(search.trim()); }}>
        <Input aria-label="Cari pelanggan tujuan" placeholder="Nama atau kode tujuan" maxLength={256} value={search}
          onChange={(event) => setSearch(event.target.value)} />
        <Button type="submit" variant="outline">Cari tujuan</Button>
      </form>
      {state.status === "loading" && <p role="status">Memuat pelanggan tujuan…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && candidates.length === 0 && <p className="muted">Tidak ada pelanggan tujuan dalam hasil ini.</p>}
      {state.status === "ready" && state.data.meta.next_cursor && <p className="muted">Persempit pencarian untuk melihat pelanggan lain.</p>}
      <form className="stack" onSubmit={merge}>
        <label>Pelanggan tujuan
          <select key={term} name="target" className="input" defaultValue="" required>
            <option value="" disabled>Pilih pelanggan</option>
            {candidates.map((item) => <option key={item.id} value={item.id}>{item.name}{item.code ? ` · ${item.code}` : ""}{item.is_active ? "" : " · Nonaktif"}</option>)}
          </select>
        </label>
        <label>Alasan penggabungan<textarea name="reason" className="input" rows={2} required maxLength={1000} /></label>
        {error && <ErrorNotice message={error.message} />}
        <div className="actions">
          <Button type="submit" disabled={pending || !candidates.length}>{pending ? "Menggabungkan…" : "Konfirmasi penggabungan"}</Button>
          <Button variant="outline" onClick={onCancel} disabled={pending}>Batal</Button>
        </div>
      </form>
    </div>
  );
}
