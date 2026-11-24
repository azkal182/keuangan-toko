import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { Clock, Calendar, ArrowRight, User, X } from "lucide-react";
import { api, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { MoneyDisplay } from "./components/money-display";
import { formatRupiah } from "./money";

const buckets: {
  key: Schema["AgingBucket"];
  label: string;
  desc: string;
  colorClass: string;
  bgClass: string;
}[] = [
  { key: "days_0_15", label: "0–15 Hari", desc: "Piutang baru (lancar)", colorClass: "text-slate-800", bgClass: "bg-slate-50 border-slate-200" },
  { key: "days_16_30", label: "16–30 Hari", desc: "Jatuh tempo normal", colorClass: "text-amber-800", bgClass: "bg-amber-50/50 border-amber-200" },
  { key: "days_31_45", label: "31–45 Hari", desc: "Mulai tertunda", colorClass: "text-orange-800", bgClass: "bg-orange-50/50 border-orange-200" },
  { key: "days_46_60", label: "46–60 Hari", desc: "Perhatian khusus", colorClass: "text-rose-800", bgClass: "bg-rose-50/50 border-rose-200" },
  { key: "days_over_60", label: "Lebih dari 60 Hari", desc: "Kritis / macet", colorClass: "text-rose-950 font-bold", bgClass: "bg-rose-100/60 border-rose-300" },
  { key: "unknown_date", label: "Tanggal Tidak Diketahui", desc: "Data historis tanpa tanggal", colorClass: "text-purple-800", bgClass: "bg-purple-50/50 border-purple-200" },
  { key: "future_date", label: "Tanggal Masa Depan", desc: "Pencatatan tanggal mendatang", colorClass: "text-purple-800", bgClass: "bg-purple-50/50 border-purple-200" },
];

export function ReceivablesAging() {
  const today = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Jakarta",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());

  const [searchParams, setSearchParams] = useSearchParams();
  const applied = searchParams.get("as_of") ?? today;
  const requestedBucket = searchParams.get("bucket");
  const bucket = buckets.find((item) => item.key === requestedBucket)?.key;
  const [asOf, setAsOf] = useState(applied);

  useEffect(() => setAsOf(applied), [applied]);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/reports/receivables-aging", {
          params: { query: { as_of: applied } },
          signal,
        }),
      ),
    [applied],
  );

  const { state, reload } = useResource(load);

  function applyFilters(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const next = new URLSearchParams(searchParams);
    if (asOf === today) next.delete("as_of");
    else next.set("as_of", asOf);
    next.delete("bucket");
    setSearchParams(next);
  }

  function selectBucket(value?: Schema["AgingBucket"]) {
    const next = new URLSearchParams(searchParams);
    if (value) next.set("bucket", value);
    else next.delete("bucket");
    setSearchParams(next);
  }

  return (
    <div className="space-y-6">
      <Card className="border-slate-200/90 shadow-2xs">
        <CardHeader className="border-b border-slate-100 pb-4">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
            <div>
              <div className="flex items-center gap-2">
                <Clock className="h-4 w-4 text-emerald-800" />
                <CardTitle className="text-base font-bold text-slate-900">
                  Laporan Umur Piutang (Aging Report)
                </CardTitle>
              </div>
              <p className="text-xs text-slate-500 mt-0.5">
                Distribusi saldo piutang terbuka berdasarkan rentang umur kalender per tanggal acuan.
              </p>
            </div>

            {/* As Of Form */}
            <form onSubmit={applyFilters} className="flex items-center gap-2">
              <div className="flex items-center gap-1.5 text-xs">
                <Calendar className="h-3.5 w-3.5 text-slate-400" />
                <span className="text-slate-500">Per Tanggal:</span>
                <Input
                  type="date"
                  value={asOf}
                  max={today}
                  onChange={(e) => setAsOf(e.target.value)}
                  className="h-8 text-xs w-36"
                />
              </div>
              <Button type="submit" variant="secondary" size="sm" className="h-8 text-xs">
                Terapkan
              </Button>
            </form>
          </div>
        </CardHeader>

        <CardContent className="p-6 space-y-6">
          {state.status === "loading" && (
            <div className="py-12 text-center text-sm text-slate-500">
              Memuat laporan umur piutang…
            </div>
          )}

          {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}

          {state.status === "ready" && (
            <>
              {/* Grand Total Highlight */}
              <div className="p-6 rounded-2xl bg-gradient-to-br from-slate-900 to-slate-800 text-white shadow-sm flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div>
                  <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                    Total Piutang Terbuka (Per {state.data.data.as_of})
                  </span>
                  <div className="mt-1">
                    <MoneyDisplay
                      amount={state.data.data.total_open_idr}
                      size="xl"
                      className="text-white text-3xl font-extrabold"
                    />
                  </div>
                  <p className="text-xs text-slate-300 mt-1">
                    Mencakup piutang terbuka dari seluruh pelanggan aktif maupun nonaktif.
                  </p>
                </div>

                <div className="text-xs text-slate-400 bg-white/10 p-3 rounded-xl border border-white/10 sm:max-w-xs">
                  Pilih salah satu kelompok umur di bawah ini untuk melihat daftar rincian pelanggan pada kelompok tersebut.
                </div>
              </div>

              {/* Bucket Grid Cards */}
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5">
                {buckets.map(({ key, label, desc, colorClass, bgClass }) => {
                  const val = state.data.data[key];
                  const isSelected = bucket === key;
                  const isZero = BigInt(val) === 0n;

                  return (
                    <button
                      key={key}
                      type="button"
                      onClick={() => selectBucket(isSelected ? undefined : key)}
                      className={`text-left p-4 rounded-xl border transition-all cursor-pointer relative overflow-hidden group ${
                        isSelected
                          ? "ring-2 ring-emerald-700 bg-emerald-50/40 border-emerald-300 shadow-sm"
                          : `${bgClass} hover:shadow-2xs`
                      }`}
                    >
                      <div className="flex items-center justify-between text-xs text-slate-500 mb-1">
                        <span className="font-semibold text-slate-700">{label}</span>
                        <ArrowRight
                          className={`h-3.5 w-3.5 transition-transform ${
                            isSelected ? "rotate-90 text-emerald-800" : "text-slate-400 group-hover:translate-x-0.5"
                          }`}
                        />
                      </div>

                      <div className={`text-lg font-bold font-mono tracking-tight ${colorClass} ${isZero ? "text-slate-400" : ""}`}>
                        {formatRupiah(val)}
                      </div>

                      <p className="text-[11px] text-slate-500 mt-1">{desc}</p>
                    </button>
                  );
                })}
              </div>

              {/* Bucket Drill-Down List */}
              {bucket && (
                <AgingCustomerList
                  key={`${applied}-${bucket}`}
                  asOf={applied}
                  bucket={bucket}
                  label={buckets.find((item) => item.key === bucket)?.label ?? "Rincian"}
                  onClose={() => selectBucket(undefined)}
                />
              )}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function AgingCustomerList({
  asOf,
  bucket,
  label,
  onClose,
}: {
  asOf: string;
  bucket: Schema["AgingBucket"];
  label: string;
  onClose: () => void;
}) {
  const [cursor, setCursor] = useState<string>();
  const [previous, setPrevious] = useState<string[]>([]);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/reports/receivables-aging/customers", {
          params: { query: { as_of: asOf, bucket, cursor } },
          signal,
        }),
      ),
    [asOf, bucket, cursor],
  );

  const { state, reload } = useResource(load);

  return (
    <Card className="border-emerald-200 bg-emerald-50/20 shadow-xs animate-in fade-in">
      <CardHeader className="border-b border-emerald-200/60 pb-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <CardTitle className="text-sm font-bold text-emerald-950 flex items-center gap-2">
              <User className="h-4 w-4" />
              <span>Daftar Pelanggan pada Kelompok: {label}</span>
            </CardTitle>
            <p className="text-xs text-slate-500 mt-0.5">
              Rincian nominal piutang terbuka untuk kelompok ini per {asOf}.
            </p>
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="h-8 gap-1 text-xs text-slate-600 hover:text-slate-900"
          >
            <X className="h-3.5 w-3.5" />
            <span>Tutup Rincian</span>
          </Button>
        </div>
      </CardHeader>

      <CardContent className="p-0">
        {state.status === "loading" && (
          <div className="p-8 text-center text-xs text-slate-500">Memuat rincian pelanggan…</div>
        )}

        {state.status === "error" && (
          <div className="p-4">
            <ErrorNotice message={state.error.message} retry={reload} />
          </div>
        )}

        {state.status === "ready" && state.data.data.length === 0 && (
          <div className="p-8 text-center text-xs text-slate-500">
            Tidak ada piutang terbuka pada kelompok umur ini.
          </div>
        )}

        {state.status === "ready" && state.data.data.length > 0 && (
          <div className="relative w-full overflow-x-auto">
            <table className="w-full text-left text-xs border-collapse">
              <thead className="bg-emerald-100/40 border-b border-emerald-200/60 text-slate-700 font-semibold uppercase text-[11px]">
                <tr>
                  <th className="p-3 pl-6">Nama Pelanggan</th>
                  <th className="p-3 pr-6 text-right">Nominal Piutang Terbuka</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-emerald-100/60 bg-white">
                {state.data.data.map((cust) => (
                  <tr key={cust.customer_id} className="hover:bg-emerald-50/50 transition-colors">
                    <td className="p-3 pl-6">
                      <Link
                        to={`/customers/${cust.customer_id}`}
                        className="font-semibold text-emerald-900 hover:underline flex items-center gap-1.5"
                      >
                        <span>{cust.customer_name}</span>
                        <ArrowRight className="h-3 w-3 text-slate-400" />
                      </Link>
                    </td>
                    <td className="p-3 pr-6 text-right font-mono font-bold text-slate-900">
                      {formatRupiah(cust.amount_idr)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* Pagination */}
        {(previous.length > 0 || (state.status === "ready" && state.data.meta.next_cursor)) && (
          <div className="flex items-center justify-between p-3 px-6 border-t border-emerald-200/60 bg-emerald-50/50">
            <span className="text-xs text-slate-500">Halaman {previous.length + 1}</span>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={!previous.length}
                onClick={() => {
                  const stack = previous.slice(0, -1);
                  setPrevious(stack);
                  setCursor(stack.at(-1));
                }}
                className="text-xs h-7"
              >
                Sebelumnya
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={state.status === "ready" ? !state.data.meta.next_cursor : true}
                onClick={() => {
                  const next = state.status === "ready" ? state.data.meta.next_cursor : undefined;
                  if (next) {
                    setPrevious([...previous, next]);
                    setCursor(next);
                  }
                }}
                className="text-xs h-7"
              >
                Berikutnya
              </Button>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
