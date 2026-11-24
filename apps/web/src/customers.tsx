import { useCallback, useEffect, useState, type FormEvent } from "react";
import { api, failure, readResult, useResource, type ApiFailure, type Schema } from "./api";
import { Brand, ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Ledger, formatRupiah } from "./ledger";
import { SalesPanel } from "./sales";
import { PaymentsPanel } from "./payments";
import { ReceivablesAging } from "./reports";
import { ImportWorkspace } from "./imports";
import { Dialog } from "./components/ui/dialog";

const labels: Record<Schema["UserRole"], string> = { owner: "Pemilik", operator: "Operator", viewer: "Pembaca" };

function selectedId() {
  return new URLSearchParams(location.search).get("pelanggan") ?? undefined;
}

type CustomerSummary = Schema["CustomerList"]["items"][number];
function unpackCustomer(value: CustomerSummary | Schema["Customer"]) {
  if ("customer" in value) return {
    customer: value.customer,
    outstanding: value.outstanding_idr,
    credit: value.customer_credit_idr,
  };
  // Older API processes return Customer directly. Keep the UI alive during a rolling restart.
  return { customer: value, outstanding: undefined, credit: undefined };
}

export function CustomerWorkspace({ session, onLogout }: {
  session: Schema["SessionDetails"];
  onLogout: () => void;
}) {
  const [selected, setSelected] = useState(selectedId);
  const [saleId, setSaleId] = useState(() => new URLSearchParams(location.search).get("penjualan") ?? undefined);
  const [creating, setCreating] = useState(false);
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState("");
  const [logoutError, setLogoutError] = useState<string>();
  const [showReport, setShowReport] = useState(false);
  const [showImports, setShowImports] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const sessionHeader = { "X-CSRF-Token": session.csrf_token };
  const canWrite = session.user.role !== "viewer";

  useEffect(() => {
    function onPopState() {
      setSelected(selectedId());
      setSaleId(new URLSearchParams(location.search).get("penjualan") ?? undefined);
      setCreating(false);
      setNotice("");
    }
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);

  function select(id?: string) {
    const url = new URL(location.href);
    if (id) url.searchParams.set("pelanggan", id);
    else url.searchParams.delete("pelanggan");
    url.searchParams.delete("penjualan");
    setSaleId(undefined);
    if (url.href !== location.href) history.pushState(null, "", url);
    setSelected(id);
    setCreating(false);
    setNotice("");
  }

  function openSale(id?: string) {
    const url = new URL(location.href);
    if (id) url.searchParams.set("penjualan", id);
    else url.searchParams.delete("penjualan");
    if (url.href !== location.href) history.pushState(null, "", url);
    setSaleId(id);
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
          <h1>{showReport ? "Laporan piutang" : showImports ? "Import workbook" : "Pelanggan"}</h1>
          <div className="pagination">
            <Button variant="outline" aria-pressed={!showReport && !showImports} onClick={() => {setShowImports(false);setShowReport(false);select(undefined);}}>Pelanggan</Button>
            <Button variant="outline" aria-pressed={showReport} onClick={() => {setShowImports(false);setShowReport(true);}}>Umur piutang</Button>
            {canWrite && <Button variant="outline" aria-pressed={showImports} onClick={() => {setShowReport(false);setShowImports(true);}}>Import</Button>}
            {!showReport && !showImports && canWrite && <Button onClick={() => { setCreating(true); setNotice(""); }}>Tambah pelanggan</Button>}
          </div>
        </div>
        {logoutError && <ErrorNotice message={logoutError} />}
        {notice && <p className="notice-success" role="status">{notice}</p>}
        {showReport ? <ReceivablesAging /> : showImports ? <ImportWorkspace csrf={session.csrf_token} owner={session.user.role === "owner"} /> : selected ? (
          <div className="customer-detail-page stack">
            <Button variant="outline" className="back-to-customers" onClick={() => select(undefined)}>← Ringkasan pelanggan</Button>
            <CustomerDetail
              key={`${selected}:${revision}`}
              id={selected}
              csrf={session.csrf_token}
              canWrite={canWrite}
              canMerge={session.user.role === "owner"}
              saleId={saleId}
              openSale={openSale}
              onDataChanged={() => setRevision((value) => value + 1)}
              onSaved={() => {
                setRevision((value) => value + 1);
                setNotice("Perubahan pelanggan berhasil disimpan.");
              }}
              onMerged={(id) => {
                select(id);
                setRevision((value) => value + 1);
                setNotice("Pelanggan berhasil digabung.");
              }}
            />
          </div>
        ) : <CustomerList onSelect={select} revision={revision} />}
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

function CustomerList({ onSelect, revision }: {
  onSelect: (id: string) => void;
  revision: number;
}) {
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState<{ search: string; cursors: string[] }>({ search: "", cursors: [] });
  const cursor = query.cursors.at(-1);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers", {
    params: { query: { search: query.search, cursor } }, signal,
  })), [query, cursor, revision]);
  const { state, reload } = useResource(load);
  return (
    <section className="panel customer-list stack" aria-label="Daftar pelanggan">
      <div><h2>Ringkasan pelanggan</h2><p className="muted">Pilih nama pelanggan untuk membuka buku piutang dan riwayat transaksinya.</p></div>
      <form className="search-form" onSubmit={(event) => {
        event.preventDefault();
        setQuery({ search: search.trim(), cursors: [] });
      }}>
        <Input aria-label="Nama atau kode pelanggan" placeholder="Nama atau kode" type="search" value={search} maxLength={256}
          onChange={(event) => setSearch(event.target.value)} />
        <Button type="submit" variant="outline">Cari</Button>
      </form>
      {state.status === "loading" && <p role="status">Memuat pelanggan…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && (
        <>
          {state.data.data.items.length === 0 ? <p className="muted">Tidak ada pelanggan yang ditemukan.</p> : (
            <div className="table-scroll" tabIndex={0} role="region" aria-label="Ringkasan saldo pelanggan">
              <table className="customer-summary-table"><thead><tr><th scope="col">Pelanggan</th><th scope="col" className="number">Piutang</th><th scope="col" className="number">Saldo kredit</th></tr></thead>
                <tbody>{state.data.data.items.map((item) => {
                  const { customer, outstanding, credit } = unpackCustomer(item as CustomerSummary | Schema["Customer"]);
                  return <tr key={customer.id}>
                    <td><button type="button" className="customer-summary-link" onClick={() => onSelect(customer.id)}><strong>{customer.name}</strong><span className="table-note">{customer.code ?? "Tanpa kode"} {!customer.is_active && " · Nonaktif"}</span></button></td>
                    <td className="number">{outstanding === undefined ? "—" : formatRupiah(outstanding)}</td><td className="number">{credit === undefined ? "—" : formatRupiah(credit)}</td>
                  </tr>;
                })}</tbody>
              </table>
            </div>
          )}
          {(query.cursors.length > 0 || state.data.meta.next_cursor) && (
            <div className="pagination">
              <Button variant="outline" disabled={!query.cursors.length} onClick={() => setQuery({ ...query, cursors: query.cursors.slice(0, -1) })}>Sebelumnya</Button>
              <Button variant="outline" disabled={!state.data.meta.next_cursor} onClick={() => {
                const next = state.data.meta.next_cursor;
                if (next) setQuery({ ...query, cursors: [...query.cursors, next] });
              }}>Berikutnya</Button>
            </div>
          )}
        </>
      )}
    </section>
  );
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
      <PaymentsPanel customer={customer} csrf={csrf} canWrite={canWrite} owner={canMerge} onChanged={onDataChanged} />
      <Ledger id={id} openSale={openSale} csrf={csrf} owner={canMerge} onChanged={onDataChanged} />
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
