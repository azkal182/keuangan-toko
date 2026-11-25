import { lazy, Suspense, useCallback, useEffect, useState, type FormEvent } from "react";
import { Navigate, Route, Routes, useLocation, useNavigate, useParams, useSearchParams } from "react-router-dom";
import {
  ArrowLeft,
  CheckCircle2,
  Clock,
  Edit3,
  GitMerge,
  Plus,
  Search,
  UserCheck,
  UserX,
  Users,
  X,
} from "lucide-react";
import { api, failure, readResult, useResource, type ApiFailure, type Schema } from "./api";
import { AppShell } from "./components/app-shell";
import { ErrorNotice, EmptyState } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { Dialog } from "./components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { MoneyDisplay } from "./components/money-display";
import { formatRupiah } from "./money";
import { RecordSaleDialog } from "./components/record-sale-dialog";

const SalesPanel = lazy(() => import("./sales").then((module) => ({ default: module.SalesPanel })));
const PaymentsPanel = lazy(() => import("./payments").then((module) => ({ default: module.PaymentsPanel })));
const Ledger = lazy(() => import("./ledger").then((module) => ({ default: module.Ledger })));
const ReceivablesAging = lazy(() => import("./reports").then((module) => ({ default: module.ReceivablesAging })));
const ImportWorkspace = lazy(() => import("./imports").then((module) => ({ default: module.ImportWorkspace })));
const UserManagementPage = lazy(() => import("./users").then((module) => ({ default: module.UserManagementPage })));
const ChangePasswordPage = lazy(() => import("./users").then((module) => ({ default: module.ChangePasswordPage })));

const agingLabels = [
  "0–15 hari",
  "16–30 hari",
  "31–45 hari",
  "46–60 hari",
  "Lebih dari 60 hari",
  "Tanggal perlu ditinjau",
];

type CustomerSummary = Schema["CustomerList"]["items"][number];

function unpackCustomer(value: CustomerSummary | Schema["Customer"]) {
  if ("customer" in value) {
    return {
      customer: value.customer,
      outstanding: value.outstanding_idr,
      aging: [
        value.days_0_15_idr,
        value.days_16_30_idr,
        value.days_31_45_idr,
        value.days_46_60_idr,
        value.days_over_60_idr,
      ],
      exceptions: BigInt(value.unknown_date_idr) + BigInt(value.future_date_idr),
    };
  }
  return { customer: value, outstanding: undefined, aging: undefined, exceptions: undefined };
}

export function CustomerWorkspace({
  session,
  onLogout,
  onPasswordChanged,
}: {
  session: Schema["SessionDetails"];
  onLogout: () => void;
  onPasswordChanged: () => void;
}) {
  const location = useLocation();
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState("");
  const canWrite = session.user.role !== "viewer";
  useEffect(() => setCreating(false), [location.pathname]);

  if (session.must_change_password) {
    if (location.pathname !== "/settings/account") return <Navigate to="/settings/account" replace />;
    return <AppShell session={session} onLogout={onLogout} passwordChangeRequired>
      <ChangePasswordPage csrf={session.csrf_token} onChanged={onPasswordChanged} />
    </AppShell>;
  }

  function select(id?: string) {
    const from = `${location.pathname}${location.search}`;
    navigate(id ? `/customers/${id}` : "/customers", id ? { state: { from } } : undefined);
    setCreating(false);
    setNotice("");
  }

  const addCustomerBtn = location.pathname === "/customers" && canWrite ? (
    <Button
      onClick={() => {
        setCreating(true);
        setNotice("");
      }}
      className="gap-2 shadow-sm text-xs font-semibold"
    >
      <Plus className="h-4 w-4" />
      <span>Tambah Pelanggan</span>
    </Button>
  ) : undefined;

  return (
    <AppShell session={session} onLogout={onLogout} actionButton={addCustomerBtn}>
      {notice && (
        <div
          role="status"
          className="flex items-center gap-2 p-3.5 rounded-xl border border-emerald-200 bg-emerald-50 text-emerald-900 text-sm font-medium shadow-2xs animate-in fade-in"
        >
          <CheckCircle2 className="h-4 w-4 text-emerald-600 shrink-0" />
          <span>{notice}</span>
        </div>
      )}

      <Suspense
        fallback={
          <div className="flex items-center justify-center p-12 text-sm text-slate-500">
            <div className="flex items-center gap-3">
              <div className="h-5 w-5 border-2 border-emerald-700 border-t-transparent rounded-full animate-spin" />
              <span>Memuat data…</span>
            </div>
          </div>
        }
      >
        <Routes>
          <Route path="customers" element={<CustomerList onSelect={select} revision={revision} />} />
          <Route
            path="customers/:customerId"
            element={
              <CustomerDetailRoute
                csrf={session.csrf_token}
                canWrite={canWrite}
                canMerge={session.user.role === "owner"}
                revision={revision}
                onChanged={() => setRevision((value) => value + 1)}
                onNotice={setNotice}
              />
            }
          />
          <Route
            path="customers/:customerId/sales/:saleId"
            element={
              <CustomerDetailRoute
                csrf={session.csrf_token}
                canWrite={canWrite}
                canMerge={session.user.role === "owner"}
                revision={revision}
                onChanged={() => setRevision((value) => value + 1)}
                onNotice={setNotice}
              />
            }
          />
          <Route path="reports/aging" element={<ReceivablesAging />} />
          <Route
            path="settings/users"
            element={session.user.role === "owner"
              ? <UserManagementPage csrf={session.csrf_token} currentUserId={session.user.id} />
              : <Navigate to="/customers" replace />}
          />
          <Route path="settings/account" element={<ChangePasswordPage csrf={session.csrf_token} onChanged={onPasswordChanged} />} />
          <Route
            path="imports"
            element={
              canWrite ? (
                <ImportWorkspace csrf={session.csrf_token} owner={session.user.role === "owner"} />
              ) : (
                <Navigate to="/customers" replace />
              )
            }
          />
          <Route
            path="imports/:batchId"
            element={
              canWrite ? (
                <ImportWorkspace csrf={session.csrf_token} owner={session.user.role === "owner"} />
              ) : (
                <Navigate to="/customers" replace />
              )
            }
          />
          <Route path="" element={<LegacyCustomerRedirect />} />
          <Route path="*" element={<Navigate to="/customers" replace />} />
        </Routes>
      </Suspense>

      <Dialog open={creating} onOpenChange={setCreating} title="Tambah Pelanggan Baru">
        <CustomerForm
          csrf={session.csrf_token}
          onCancel={() => setCreating(false)}
          onSaved={(customer) => {
            select(customer.id);
            setCreating(false);
            setRevision((value) => value + 1);
            setNotice(`Pelanggan "${customer.name}" berhasil ditambahkan.`);
          }}
        />
      </Dialog>
    </AppShell>
  );
}

function LegacyCustomerRedirect() {
  const [searchParams] = useSearchParams();
  const customerId = searchParams.get("pelanggan");
  const saleId = searchParams.get("penjualan");
  if (!customerId) return <Navigate to="/customers" replace />;
  return <Navigate to={saleId ? `/customers/${customerId}/sales/${saleId}` : `/customers/${customerId}`} replace />;
}

function CustomerList({ onSelect, revision }: { onSelect: (id: string) => void; revision: number }) {
  const [searchParams, setSearchParams] = useSearchParams();
  const appliedSearch = searchParams.get("search") ?? "";
  const pageCursors = searchParams.getAll("page_cursor");
  const cursor = pageCursors.at(-1);
  const [search, setSearch] = useState(appliedSearch);

  useEffect(() => setSearch(appliedSearch), [appliedSearch]);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers", {
          params: { query: { search: appliedSearch, cursor } },
          signal,
        }),
      ),
    [appliedSearch, cursor, revision],
  );

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
    const cursors =
      direction === "previous"
        ? pageCursors.slice(0, -1)
        : nextCursor
        ? [...pageCursors, nextCursor]
        : pageCursors;
    const next = new URLSearchParams(searchParams);
    next.delete("page_cursor");
    for (const value of cursors) next.append("page_cursor", value);
    setSearchParams(next);
  }

  // Calculate high-level summary KPIs from current page
  const items = state.status === "ready" ? state.data.data.items : [];
  const totalOutstandingPage = items.reduce((sum, item) => {
    const unpacked = unpackCustomer(item as CustomerSummary | Schema["Customer"]);
    return sum + (unpacked.outstanding ? BigInt(unpacked.outstanding) : 0n);
  }, 0n);
  const totalOver60Page = items.reduce((sum, item) => {
    const unpacked = unpackCustomer(item as CustomerSummary | Schema["Customer"]);
    return sum + (unpacked.aging?.[4] ? BigInt(unpacked.aging[4]) : 0n);
  }, 0n);

  return (
    <div className="space-y-6">
      {/* KPI Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="border-slate-200/90 shadow-2xs">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Total Pelanggan Terdaftar</span>
              <Users className="h-4 w-4 text-emerald-700" />
            </div>
            <CardTitle className="text-2xl font-bold font-mono text-slate-900 mt-1">
              {items.length} <span className="text-xs font-normal text-slate-500">di halaman ini</span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">Aktif dan terpantau dalam buku piutang</p>
          </CardContent>
        </Card>

        <Card className="border-slate-200/90 shadow-2xs bg-gradient-to-br from-white to-emerald-50/20">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Total Piutang Terbuka</span>
              <div className="h-2 w-2 rounded-full bg-emerald-600 animate-pulse" />
            </div>
            <CardTitle className="text-2xl font-bold font-mono text-emerald-900 mt-1">
              <MoneyDisplay amount={totalOutstandingPage.toString()} size="xl" variant="neutral" />
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">Total piutang belum terbayar</p>
          </CardContent>
        </Card>

        <Card className="border-slate-200/90 shadow-2xs bg-gradient-to-br from-white to-rose-50/20">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Piutang Kritis (&gt;60 Hari)</span>
              <Clock className="h-4 w-4 text-rose-600" />
            </div>
            <CardTitle className="text-2xl font-bold font-mono text-rose-700 mt-1">
              <MoneyDisplay amount={totalOver60Page.toString()} size="xl" variant="danger" />
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">Memerlukan penagihan intensif</p>
          </CardContent>
        </Card>

        <Card className="border-slate-200/90 shadow-2xs">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between text-slate-500">
              <span className="text-xs font-semibold">Basis Zona Waktu</span>
              <Badge variant="outline" className="text-[10px]">Asia/Jakarta</Badge>
            </div>
            <CardTitle className="text-base font-semibold text-slate-800 mt-1">
              Kalender Usaha
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-slate-500">5 bucket umur piutang dihitung otomatis</p>
          </CardContent>
        </Card>
      </div>

      {/* Main Table Card */}
      <Card className="shadow-2xs border-slate-200/90">
        <CardHeader className="border-b border-slate-100 pb-4">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <CardTitle className="text-base font-bold text-slate-900">Buku Ringkasan Saldo Pelanggan</CardTitle>
              <p className="text-xs text-slate-500 mt-0.5">
                Klik pada nama pelanggan untuk membuka detail buku piutang, penjualan kredit, dan pembayaran.
              </p>
            </div>

            {/* Search form */}
            <form onSubmit={applySearch} className="flex items-center gap-2 max-w-sm w-full">
              <div className="relative flex-1">
                <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
                <Input
                  aria-label="Nama atau kode pelanggan"
                  placeholder="Cari nama atau kode pelanggan…"
                  type="search"
                  value={search}
                  maxLength={256}
                  onChange={(event) => setSearch(event.target.value)}
                  className={`pl-9 ${search ? "pr-8" : ""} h-9 text-xs`}
                />
                {search && (
                  <button
                    type="button"
                    title="Hapus pencarian"
                    onClick={() => {
                      setSearch("");
                      const next = new URLSearchParams(searchParams);
                      next.delete("search");
                      next.delete("page_cursor");
                      setSearchParams(next);
                    }}
                    className="absolute right-2.5 top-2.5 text-slate-400 hover:text-slate-600 transition-colors"
                  >
                    <X className="h-4 w-4" />
                  </button>
                )}
              </div>
              <Button type="submit" variant="secondary" size="sm" className="shrink-0 text-xs font-semibold">
                Cari
              </Button>
            </form>
          </div>
        </CardHeader>

        <CardContent className="p-0">
          {state.status === "loading" && (
            <div className="p-12 text-center text-sm text-slate-500">
              <div className="flex items-center justify-center gap-3">
                <div className="h-5 w-5 border-2 border-emerald-700 border-t-transparent rounded-full animate-spin" />
                <span>Memuat ringkasan saldo pelanggan…</span>
              </div>
            </div>
          )}

          {state.status === "error" && (
            <div className="p-6">
              <ErrorNotice message={state.error.message} retry={reload} />
            </div>
          )}

          {state.status === "ready" && items.length === 0 && (
            <EmptyState
              title="Tidak ada pelanggan yang ditemukan"
              description="Coba ubah kata kunci pencarian atau daftarkan pelanggan baru."
            />
          )}

          {state.status === "ready" && items.length > 0 && (
            <div className="relative w-full overflow-x-auto">
              <table className="w-full text-left text-xs border-collapse">
                <thead>
                  <tr className="border-b border-slate-200 bg-slate-50/90 text-slate-600 font-semibold tracking-wider uppercase text-[11px]">
                    <th scope="col" className="p-3.5 pl-6 sticky left-0 z-10 bg-slate-50 border-r border-slate-200/80 shadow-2xs min-w-[14rem]">
                      Pelanggan
                    </th>
                    <th scope="col" className="p-3.5 text-right font-semibold text-slate-900 min-w-[8rem]">
                      Total Piutang
                    </th>
                    <th scope="col" className="p-3.5 text-right min-w-[7rem]">0–15 Hari</th>
                    <th scope="col" className="p-3.5 text-right min-w-[7rem]">16–30 Hari</th>
                    <th scope="col" className="p-3.5 text-right min-w-[7rem]">31–45 Hari</th>
                    <th scope="col" className="p-3.5 text-right min-w-[7rem]">46–60 Hari</th>
                    <th scope="col" className="p-3.5 text-right min-w-[8rem] text-rose-700 font-bold">
                      &gt;60 Hari
                    </th>
                    <th scope="col" className="p-3.5 pr-6 text-right min-w-[8rem] text-purple-700">
                      Perlu Ditinjau
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {items.map((item) => {
                    const { customer, outstanding, aging, exceptions } = unpackCustomer(
                      item as CustomerSummary | Schema["Customer"],
                    );
                    const isCritical = aging?.[4] && BigInt(aging[4]) > 0n;

                    return (
                      <tr
                        key={customer.id}
                        onClick={() => onSelect(customer.id)}
                        className="group hover:bg-emerald-50/40 transition-colors cursor-pointer"
                      >
                        <td className="p-3.5 pl-6 sticky left-0 z-1 bg-white group-hover:bg-emerald-50/40 border-r border-slate-200/80 transition-colors">
                          <div className="flex flex-col">
                            <span className="font-semibold text-slate-900 group-hover:text-emerald-950 text-sm">
                              {customer.name}
                            </span>
                            <div className="flex items-center gap-2 mt-0.5">
                              <span className="font-mono text-[11px] text-slate-500 font-medium">
                                {customer.code ?? "Tanpa kode"}
                              </span>
                              {!customer.is_active && (
                                <Badge variant="secondary" className="text-[10px] py-0 px-1.5 h-4">
                                  Nonaktif
                                </Badge>
                              )}
                            </div>
                          </div>
                        </td>

                        <td className="p-3.5 text-right font-mono font-bold text-slate-900 text-sm">
                          {outstanding === undefined ? "—" : formatRupiah(outstanding)}
                        </td>

                        {aging?.map((amount, idx) => {
                          const isNonZero = BigInt(amount) > 0n;
                          const isWarning = idx === 3 && isNonZero;
                          const isDanger = idx === 4 && isNonZero;

                          return (
                            <td
                              key={idx}
                              className={`p-3.5 text-right font-mono text-xs tabular-nums ${
                                isDanger
                                  ? "font-bold text-rose-700 bg-rose-50/30"
                                  : isWarning
                                  ? "font-medium text-amber-800"
                                  : isNonZero
                                  ? "text-slate-800"
                                  : "text-slate-400"
                              }`}
                            >
                              {formatRupiah(amount)}
                            </td>
                          );
                        })}

                        {aging && (
                          <td
                            className={`p-3.5 pr-6 text-right font-mono text-xs tabular-nums ${
                              exceptions && exceptions > 0n
                                ? "font-semibold text-purple-700 bg-purple-50/30"
                                : "text-slate-400"
                            }`}
                            title="Nominal piutang dengan tanggal transaksi tidak diketahui atau berada di masa depan"
                          >
                            {formatRupiah(exceptions!.toString())}
                          </td>
                        )}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          {/* Pagination */}
          {(pageCursors.length > 0 || (state.status === "ready" && state.data.meta.next_cursor)) && (
            <div className="flex items-center justify-between p-4 border-t border-slate-100 bg-slate-50/50">
              <span className="text-xs text-slate-500">
                Halaman {pageCursors.length + 1}
              </span>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!pageCursors.length}
                  onClick={() => movePage("previous")}
                  className="text-xs"
                >
                  Sebelumnya
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={state.status === "ready" ? !state.data.meta.next_cursor : true}
                  onClick={() =>
                    movePage(
                      "next",
                      state.status === "ready" ? state.data.meta.next_cursor ?? undefined : undefined,
                    )
                  }
                  className="text-xs"
                >
                  Berikutnya
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function CustomerDetailRoute({
  csrf,
  canWrite,
  canMerge,
  revision,
  onChanged,
  onNotice,
}: {
  csrf: string;
  canWrite: boolean;
  canMerge: boolean;
  revision: number;
  onChanged: () => void;
  onNotice: (message: string) => void;
}) {
  const { customerId, saleId } = useParams();
  const navigate = useNavigate();
  const location = useLocation();

  if (!customerId) return <Navigate to="/customers" replace />;
  const returnPath = (location.state as { from?: unknown } | null)?.from;
  const backToList =
    typeof returnPath === "string" && returnPath.startsWith("/customers") && !returnPath.startsWith("//")
      ? returnPath
      : "/customers";

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => navigate(backToList)}
          className="gap-2 text-xs font-semibold text-slate-700 hover:text-slate-900"
        >
          <ArrowLeft className="h-4 w-4" />
          <span>Kembali ke Ringkasan Pelanggan</span>
        </Button>
      </div>

      <CustomerDetail
        key={`${customerId}:${revision}`}
        id={customerId}
        csrf={csrf}
        canWrite={canWrite}
        canMerge={canMerge}
        saleId={saleId}
        openSale={(id) =>
          navigate(id ? `/customers/${customerId}/sales/${id}` : `/customers/${customerId}`, {
            state: location.state,
          })
        }
        onDataChanged={onChanged}
        onNotice={onNotice}
        onSaved={() => {
          onChanged();
          onNotice("Perubahan data pelanggan berhasil disimpan.");
        }}
        onMerged={(id) => {
          navigate(`/customers/${id}`, { state: location.state });
          onChanged();
          onNotice("Pelanggan berhasil digabungkan.");
        }}
      />
    </div>
  );
}

function CustomerDetail({
  id,
  csrf,
  canWrite,
  canMerge,
  saleId,
  openSale,
  onSaved,
  onDataChanged,
  onMerged,
  onNotice,
}: {
  id: string;
  csrf: string;
  canWrite: boolean;
  canMerge: boolean;
  saleId?: string;
  openSale?: (id?: string) => void;
  onSaved: () => void;
  onDataChanged: () => void;
  onMerged: (id: string) => void;
  onNotice?: (message: string) => void;
}) {
  const [mode, setMode] = useState<"view" | "edit" | "merge">("view");
  const [activeTab, setActiveTab] = useState<"ledger" | "payments">("ledger");
  const [recordSaleOpen, setRecordSaleOpen] = useState(false);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers/{id}", {
          params: { path: { id } },
          signal,
        }),
      ).data,
    [id],
  );

  const { state, reload } = useResource(load);

  if (state.status === "loading") {
    return (
      <div className="flex items-center justify-center p-16 text-sm text-slate-500">
        <div className="flex items-center gap-3">
          <div className="h-5 w-5 border-2 border-emerald-700 border-t-transparent rounded-full animate-spin" />
          <span>Memuat detail pelanggan…</span>
        </div>
      </div>
    );
  }

  if (state.status === "error") {
    return <ErrorNotice message={state.error.message} retry={reload} />;
  }

  const { customer, outstanding_idr, customer_credit_idr } = state.data;
  const isLunas = BigInt(outstanding_idr) === 0n;

  return (
    <div className="space-y-6">
      {/* Customer Header Card */}
      <Card className="border-slate-200/90 shadow-2xs overflow-hidden">
        <div className="bg-gradient-to-r from-slate-900 to-slate-800 p-6 text-white">
          <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-4">
            <div>
              <div className="flex items-center gap-3">
                <h2 className="text-2xl font-bold tracking-tight text-white">{customer.name}</h2>
                <Badge
                  variant={customer.is_active ? "success" : "secondary"}
                  className="gap-1 text-xs px-2.5 py-0.5"
                >
                  {customer.is_active ? (
                    <>
                      <UserCheck className="h-3 w-3" />
                      <span>Aktif</span>
                    </>
                  ) : (
                    <>
                      <UserX className="h-3 w-3" />
                      <span>Nonaktif</span>
                    </>
                  )}
                </Badge>
              </div>

              <div className="flex flex-wrap items-center gap-2 mt-2 text-xs text-slate-300">
                <span className="font-mono bg-white/10 px-2 py-0.5 rounded font-medium">
                  {customer.code ?? "Tanpa Kode"}
                </span>
                <span>•</span>
                <span>{customer.address ?? "Alamat belum tercatat"}</span>
              </div>
            </div>

            {/* Quick Actions */}
            {mode === "view" && (canWrite || canMerge) && (
              <div className="flex items-center gap-2">
                {canWrite && (
                  <Button
                    size="sm"
                    onClick={() => setRecordSaleOpen(true)}
                    className="gap-1.5 bg-emerald-600 hover:bg-emerald-500 text-white shadow-2xs text-xs font-semibold"
                  >
                    <Plus className="h-3.5 w-3.5" />
                    <span>Catat Belanja</span>
                  </Button>
                )}
                {canWrite && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setMode("edit")}
                    className="gap-1.5 bg-white/10 border-white/20 text-white hover:bg-white/20 hover:text-white"
                  >
                    <Edit3 className="h-3.5 w-3.5" />
                    <span>Ubah Data</span>
                  </Button>
                )}
                {canMerge && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setMode("merge")}
                    className="gap-1.5 bg-white/10 border-white/20 text-white hover:bg-white/20 hover:text-white"
                  >
                    <GitMerge className="h-3.5 w-3.5" />
                    <span>Gabungkan</span>
                  </Button>
                )}
              </div>
            )}
          </div>
        </div>

        {/* Financial Highlights */}
        <div className="grid grid-cols-1 sm:grid-cols-2 divide-y sm:divide-y-0 sm:divide-x divide-slate-200 bg-slate-50/50 p-6">
          <div className="sm:pr-6 pb-4 sm:pb-0 flex flex-col justify-center">
            <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">
              Total Piutang Terutang
            </span>
            <div className="flex items-baseline gap-3 mt-1.5">
              <MoneyDisplay
                amount={outstanding_idr}
                size="xl"
                variant={isLunas ? "success" : "danger"}
              />
              <Badge variant={isLunas ? "success" : "warning"} className="text-xs">
                {isLunas ? "Lunas" : "Belum Lunas"}
              </Badge>
            </div>
            <p className="text-xs text-slate-500 mt-1">Saldo kewajiban piutang dari buku piutang</p>
          </div>

          <div className="sm:pl-6 pt-4 sm:pt-0 flex flex-col justify-center">
            <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">
              Saldo Kredit Pelanggan (Deposit)
            </span>
            <div className="flex items-baseline gap-3 mt-1.5">
              <MoneyDisplay amount={customer_credit_idr} size="xl" variant="credit" />
              {BigInt(customer_credit_idr) > 0n && (
                <Badge variant="info" className="text-xs">
                  Ada Saldo Lebih
                </Badge>
              )}
            </div>
            <p className="text-xs text-slate-500 mt-1">
              Kelebihan pembayaran yang dapat diterapkan ke transaksi piutang
            </p>
          </div>
        </div>
      </Card>

      {/* Merge Form Modal */}
      {mode === "merge" && (
        <Card className="border-amber-200 bg-amber-50/40 p-6 shadow-sm">
          <MergeForm
            customer={customer}
            csrf={csrf}
            onCancel={() => setMode("view")}
            onMerged={onMerged}
          />
        </Card>
      )}

      {/* Edit Customer Dialog */}
      <Dialog
        open={mode === "edit"}
        onOpenChange={(open) => {
          if (!open) setMode("view");
        }}
        title={`Ubah Data Pelanggan: ${customer.name}`}
      >
        {mode === "edit" && (
          <CustomerForm
            key={customer.version}
            customer={customer}
            csrf={csrf}
            onCancel={() => setMode("view")}
            onReload={reload}
            onSaved={() => {
              setMode("view");
              reload();
              onSaved();
            }}
          />
        )}
      </Dialog>

      {/* Detail Workspace Tabs */}
      <Tabs
        value={activeTab}
        onValueChange={(val) => {
          setActiveTab(val as "ledger" | "payments");
        }}
      >
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-3">
          <TabsList className="bg-slate-200/70 p-1 rounded-xl">
            <TabsTrigger value="ledger" className="gap-2 text-xs">
              <span>Buku Piutang</span>
            </TabsTrigger>
            <TabsTrigger value="payments" className="gap-2 text-xs">
              <span>Riwayat Pembayaran</span>
            </TabsTrigger>
          </TabsList>

          {canWrite && (
            <Button
              onClick={() => setRecordSaleOpen(true)}
              className="gap-2 shadow-2xs text-xs font-semibold bg-emerald-700 hover:bg-emerald-800 text-white"
            >
              <Plus className="h-4 w-4" />
              <span>Catat Belanja / Piutang</span>
            </Button>
          )}
        </div>

        <TabsContent value="ledger">
          <Ledger id={id} />
        </TabsContent>

        <TabsContent value="payments">
          <PaymentsPanel
            customer={customer}
            csrf={csrf}
            canWrite={canWrite}
            onChanged={() => {
              reload();
              onDataChanged();
            }}
          />
        </TabsContent>
      </Tabs>

      {/* Instant Multi-item Sale Dialog */}
      {canWrite && (
        <RecordSaleDialog
          open={recordSaleOpen}
          onOpenChange={setRecordSaleOpen}
          customer={customer}
          csrf={csrf}
          onSuccess={(msg) => {
            reload();
            onDataChanged();
            onNotice?.(msg);
          }}
        />
      )}
    </div>
  );
}

function CustomerForm({
  customer,
  csrf,
  onSaved,
  onCancel,
  onReload,
}: {
  customer?: Schema["Customer"];
  csrf: string;
  onSaved: (customer: Schema["Customer"]) => void;
  onCancel: () => void;
  onReload?: () => void;
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
      const result = customer
        ? await api.PATCH("/api/v1/customers/{id}", {
            params: { path: { id: customer.id }, header },
            body: { ...identity, expected_version: customer.version, is_active: data.get("is_active") === "on" },
          })
        : await api.POST("/api/v1/customers", { params: { header }, body: identity });
      onSaved(readResult(result).data);
    } catch (error) {
      setError(failure(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <form className="space-y-4" onSubmit={save}>
      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="cust-name">
          Nama Pelanggan <span className="text-rose-600">*</span>
        </label>
        <Input
          id="cust-name"
          name="name"
          defaultValue={customer?.name}
          maxLength={256}
          required
          placeholder="contoh: Toko Maju Jaya"
          aria-invalid={!!error?.fields.name}
        />
        {error?.fields.name && <p className="text-xs text-rose-600">{error.fields.name}</p>}
      </div>

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="cust-code">
          Kode Pelanggan (opsional)
        </label>
        <Input
          id="cust-code"
          name="code"
          defaultValue={customer?.code ?? ""}
          maxLength={128}
          placeholder="contoh: PLG-001"
          aria-invalid={!!error?.fields.code}
        />
        {error?.fields.code && <p className="text-xs text-rose-600">{error.fields.code}</p>}
      </div>

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="cust-address">
          Alamat / Keterangan (opsional)
        </label>
        <textarea
          id="cust-address"
          name="address"
          defaultValue={customer?.address ?? ""}
          rows={3}
          maxLength={2000}
          placeholder="Alamat toko, pasar, atau catatan pengiriman…"
          className="w-full rounded-lg border border-slate-300 p-2.5 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
          aria-invalid={!!error?.fields.address}
        />
        {error?.fields.address && <p className="text-xs text-rose-600">{error.fields.address}</p>}
      </div>

      {customer && (
        <label className="flex items-center gap-2 cursor-pointer pt-1">
          <input
            type="checkbox"
            name="is_active"
            defaultChecked={customer.is_active}
            className="h-4 w-4 rounded border-slate-300 text-emerald-700 focus:ring-emerald-700"
          />
          <span className="text-xs font-medium text-slate-700">Pelanggan Aktif</span>
        </label>
      )}

      {error && <ErrorNotice message={error.message} />}

      <div className="flex items-center justify-end gap-2 pt-4 border-t border-slate-200">
        <Button variant="outline" onClick={onCancel} disabled={pending}>
          Batal
        </Button>
        {error?.fields.expected_version && onReload && (
          <Button
            variant="outline"
            onClick={() => {
              setError(undefined);
              onReload();
            }}
          >
            Muat Ulang Detail
          </Button>
        )}
        <Button type="submit" disabled={pending}>
          {pending ? "Menyimpan…" : "Simpan Data"}
        </Button>
      </div>
    </form>
  );
}

function MergeForm({
  customer,
  csrf,
  onMerged,
  onCancel,
}: {
  customer: Schema["Customer"];
  csrf: string;
  onMerged: (id: string) => void;
  onCancel: () => void;
}) {
  const [search, setSearch] = useState("");
  const [term, setTerm] = useState("");
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers", {
          params: { query: { search: term, limit: 100 } },
          signal,
        }),
      ),
    [term],
  );

  const { state, reload } = useResource(load);
  const candidates =
    state.status === "ready"
      ? state.data.data.items
          .map((item) => unpackCustomer(item as CustomerSummary | Schema["Customer"]).customer)
          .filter((item) => item.id !== customer.id)
      : [];

  async function merge(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const target_customer_id = String(data.get("target"));
    setError(undefined);
    setPending(true);
    try {
      readResult(
        await api.POST("/api/v1/customers/{id}/merge", {
          params: { path: { id: customer.id }, header: { "X-CSRF-Token": csrf } },
          body: {
            target_customer_id,
            reason: String(data.get("reason")),
            expected_version: customer.version,
          },
        }),
      );
      onMerged(target_customer_id);
    } catch (error) {
      setError(failure(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="border-b border-amber-200 pb-3">
        <h3 className="text-base font-bold text-amber-950 flex items-center gap-2">
          <GitMerge className="h-4 w-4" />
          <span>Penggabungan Pelanggan (Merge)</span>
        </h3>
        <p className="text-xs text-amber-800 mt-1">
          Seluruh riwayat buku piutang dan pembayaran pelanggan ini akan dialihkan ke pelanggan tujuan. Pelanggan ini akan dinonaktifkan secara permanen.
        </p>
      </div>

      <form
        className="flex items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          setTerm(search.trim());
        }}
      >
        <Input
          placeholder="Cari nama atau kode pelanggan tujuan…"
          maxLength={256}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="text-xs"
        />
        <Button type="submit" variant="secondary" size="sm" className="shrink-0 text-xs font-semibold">
          Cari Tujuan
        </Button>
      </form>

      {state.status === "loading" && <p className="text-xs text-slate-500">Memuat calon pelanggan tujuan…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}

      <form className="space-y-4 pt-2" onSubmit={merge}>
        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-slate-700" htmlFor="target-select">
            Pilih Pelanggan Tujuan
          </label>
          <select
            id="target-select"
            key={term}
            name="target"
            defaultValue=""
            required
            className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
          >
            <option value="" disabled>
              -- Pilih salah satu pelanggan --
            </option>
            {candidates.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name} {item.code ? `(${item.code})` : ""} {!item.is_active ? "· Nonaktif" : ""}
              </option>
            ))}
          </select>
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-slate-700" htmlFor="merge-reason">
            Alasan Penggabungan (wajib dicatat di audit)
          </label>
          <textarea
            id="merge-reason"
            name="reason"
            rows={2}
            required
            maxLength={1000}
            placeholder="contoh: Nama duplikat pada pencatatan buku lama…"
            className="w-full rounded-lg border border-slate-300 p-2 text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
          />
        </div>

        {error && <ErrorNotice message={error.message} />}

        <div className="flex items-center justify-end gap-2 pt-2">
          <Button variant="outline" size="sm" onClick={onCancel} disabled={pending}>
            Batal
          </Button>
          <Button
            type="submit"
            variant="destructive"
            size="sm"
            disabled={pending || !candidates.length}
          >
            {pending ? "Menggabungkan…" : "Konfirmasi & Gabungkan"}
          </Button>
        </div>
      </form>
    </div>
  );
}
