import { useCallback, useMemo, useState } from "react";
import { Calendar, Filter, FileText, ArrowRight, RotateCcw } from "lucide-react";
import { api, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { MoneyDisplay } from "./components/money-display";
import { formatRupiah, formatAccountingRupiah, formatQuantity } from "./money";

const businessDate = new Intl.DateTimeFormat("id-ID", {
  day: "numeric",
  month: "short",
  year: "numeric",
  timeZone: "UTC",
});

const entryLabels: Record<string, string> = {
  sale: "Penjualan kredit",
  credit_sale: "Penjualan kredit",
  payment: "Pembayaran",
  legacy_debit: "Debit historis",
  legacy_payment: "Pembayaran historis",
  reversal: "Pembalikan",
};

export function Ledger({ id, openSale }: { id: string; openSale?: (id: string) => void }) {
  const [query, setQuery] = useState<{ from?: string; to?: string; cursors: string[] }>({ cursors: [] });
  const [sortOrder, setSortOrder] = useState<"desc" | "asc">("desc");
  const cursor = query.cursors.at(-1);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers/{id}/ledger", {
          params: { path: { id }, query: { from: query.from, to: query.to, cursor, order: sortOrder } },
          signal,
        }),
      ),
    [id, query, cursor, sortOrder],
  );

  const { state, reload } = useResource(load);

  const displayEntries = useMemo(() => {
    if (state.status !== "ready") return [];
    return state.data.data.entries;
  }, [state]);

  return (
    <Card className="border-slate-200/90 shadow-2xs">
      <CardHeader className="border-b border-slate-100 pb-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2">
              <FileText className="h-4 w-4 text-emerald-800" />
              <CardTitle className="text-base font-bold text-slate-900">Buku Piutang (Ledger Mutasi)</CardTitle>
            </div>
            <p className="text-xs text-slate-500 mt-0.5">
              Riwayat kronologis mutasi penjualan kredit, pembayaran, dan saldo terutang.
            </p>
          </div>

          {/* Date Filter Bar */}
          <form
            onSubmit={(event) => {
              event.preventDefault();
              const values = new FormData(event.currentTarget);
              setQuery({
                from: String(values.get("from")) || undefined,
                to: String(values.get("to")) || undefined,
                cursors: [],
              });
            }}
            className="flex flex-wrap items-center gap-2"
          >
            <div className="flex items-center gap-1.5 text-xs">
              <Calendar className="h-3.5 w-3.5 text-slate-400" />
              <Input name="from" type="date" className="h-8 text-xs w-32" />
              <span className="text-slate-400">s/d</span>
              <Input name="to" type="date" className="h-8 text-xs w-32" />
            </div>
            <Button type="submit" variant="secondary" size="sm" className="h-8 text-xs gap-1">
              <Filter className="h-3 w-3" />
              <span>Filter</span>
            </Button>
            <Button
              type="reset"
              variant="outline"
              size="sm"
              onClick={() => setQuery({ cursors: [] })}
              className="h-8 text-xs gap-1"
            >
              <RotateCcw className="h-3 w-3" />
              <span>Reset</span>
            </Button>
          </form>
        </div>
      </CardHeader>

      <CardContent className="p-0">
        {state.status === "loading" && (
          <div className="p-12 text-center text-sm text-slate-500">
            <div className="flex items-center justify-center gap-3">
              <div className="h-5 w-5 border-2 border-emerald-700 border-t-transparent rounded-full animate-spin" />
              <span>Memuat catatan buku piutang…</span>
            </div>
          </div>
        )}

        {state.status === "error" && (
          <div className="p-6">
            <ErrorNotice message={state.error.message} retry={reload} />
          </div>
        )}

        {state.status === "ready" && (
          <>
            {/* Ledger Balance Banner & Sorting Controls */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between p-4 px-6 bg-slate-50/80 border-b border-slate-100 gap-3">
              <div className="flex items-baseline gap-2">
                <span className="text-xs font-semibold text-slate-600 uppercase tracking-wider">
                  Saldo Bersih Buku Piutang:
                </span>
                <MoneyDisplay
                  amount={state.data.data.summary.ledger_balance_idr}
                  size="lg"
                  variant={BigInt(state.data.data.summary.ledger_balance_idr) > 0n ? "danger" : "success"}
                />
              </div>

              {/* Sorting Toggle Buttons */}
              <div className="flex items-center gap-1 bg-slate-200/80 p-0.5 rounded-xl border border-slate-200 text-xs shadow-2xs">
                <button
                  type="button"
                  onClick={() => {
                    if (sortOrder !== "desc") {
                      setSortOrder("desc");
                      setQuery((q) => ({ ...q, cursors: [] }));
                    }
                  }}
                  className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                    sortOrder === "desc"
                      ? "bg-white text-emerald-950 shadow-xs"
                      : "text-slate-600 hover:text-slate-900"
                  }`}
                >
                  Terbaru di Atas
                </button>
                <button
                  type="button"
                  onClick={() => {
                    if (sortOrder !== "asc") {
                      setSortOrder("asc");
                      setQuery((q) => ({ ...q, cursors: [] }));
                    }
                  }}
                  className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                    sortOrder === "asc"
                      ? "bg-white text-emerald-950 shadow-xs"
                      : "text-slate-600 hover:text-slate-900"
                  }`}
                >
                  Terlama di Atas (Kronologis)
                </button>
              </div>
            </div>

            {displayEntries.length === 0 ? (
              <div className="p-10 text-center text-xs text-slate-500">
                Belum ada transaksi pada periode yang dipilih.
              </div>
            ) : (
              <div className="relative w-full overflow-x-auto">
                <table className="w-full text-left text-xs border-collapse">
                  <thead>
                    <tr className="border-b border-slate-200 bg-slate-50 text-slate-600 font-semibold uppercase text-[11px]">
                      <th scope="col" rowSpan={2} className="p-3 pl-6 border-r border-slate-200/80">
                        Tanggal
                      </th>
                      <th scope="col" rowSpan={2} className="p-3 min-w-[12rem]">
                        Uraian Transaksi
                      </th>
                      <th scope="col" rowSpan={2} className="p-3 text-right">
                        Jumlah (Qty)
                      </th>
                      <th scope="col" rowSpan={2} className="p-3 text-right">
                        Harga Satuan
                      </th>
                      <th scope="col" rowSpan={2} className="p-3 text-right text-amber-900 bg-amber-50/40">
                        Debet (Rp)
                      </th>
                      <th scope="col" rowSpan={2} className="p-3 text-right text-emerald-900 bg-emerald-50/40">
                        Kredit (Rp)
                      </th>
                      <th scope="col" rowSpan={2} className="p-3 text-right font-bold text-slate-900 bg-slate-100/60">
                        Saldo (Rp)
                      </th>
                      <th scope="colgroup" colSpan={3} className="p-2 text-center border-l border-slate-200/80 bg-slate-100/80">
                        Keterangan (Status Alokasi & Umur)
                      </th>
                    </tr>
                    <tr className="border-b border-slate-200 bg-slate-50/60 text-slate-500 text-[10px]">
                      <th scope="col" className="p-2 text-center border-l border-slate-200/80 min-w-[5rem]">Lunas</th>
                      <th scope="col" className="p-2 text-right min-w-[6.5rem]">Sisa</th>
                      <th scope="col" className="p-2 pr-6 text-right min-w-[6.5rem]">Umur Piutang</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100">
                    {displayEntries.map((entry) => {
                      const isDebit = entry.debit_idr !== "0";
                      const isCredit = entry.credit_idr !== "0";
                      const isFullySettled = BigInt(entry.remaining_idr) === 0n;

                      return (
                        <tr key={entry.id} className="hover:bg-slate-50/80 transition-colors">
                          <td className="p-3 pl-6 font-mono text-slate-600 whitespace-nowrap border-r border-slate-100">
                            {entry.effective_date
                              ? businessDate.format(new Date(entry.effective_date))
                              : "Tidak Diketahui"}
                          </td>

                          <td className="p-3">
                            <div className="flex flex-col">
                              {entry.entry_type === "credit_sale" && entry.description && openSale ? (
                                <button
                                  type="button"
                                  onClick={() => openSale(entry.source_id)}
                                  className="text-left font-semibold text-emerald-800 hover:text-emerald-950 hover:underline flex items-center gap-1 group"
                                >
                                  <span>{entry.description}</span>
                                  <ArrowRight className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                                </button>
                              ) : (
                                <span className="font-medium text-slate-800">
                                  {entry.description ?? entryLabels[entry.entry_type] ?? entry.entry_type}
                                </span>
                              )}
                              {entry.is_reversed && (
                                <Badge variant="destructive" className="w-fit text-[10px] py-0 px-1 mt-0.5">
                                  Dibalik (Reversed)
                                </Badge>
                              )}
                            </div>
                          </td>

                          <td className="p-3 text-right font-mono text-slate-700">
                            {formatQuantity(entry.quantity)}
                          </td>

                          <td className="p-3 text-right font-mono text-slate-600">
                            {entry.unit_price_idr == null ? "—" : formatRupiah(entry.unit_price_idr)}
                          </td>

                          <td className={`p-3 text-right font-mono tabular-nums ${isDebit ? "font-bold text-amber-900 bg-amber-50/20" : "text-slate-400"}`}>
                            {isDebit ? formatRupiah(entry.debit_idr) : "—"}
                          </td>

                          <td className={`p-3 text-right font-mono tabular-nums ${isCredit ? "font-bold text-emerald-800 bg-emerald-50/20" : "text-slate-400"}`}>
                            {isCredit ? formatRupiah(entry.credit_idr) : "—"}
                          </td>

                          <td className={`p-3 text-right font-mono tabular-nums font-bold ${BigInt(entry.running_balance_idr) < 0n ? "text-rose-700 bg-rose-50/30" : "text-slate-900 bg-slate-50/40"}`}>
                            {formatAccountingRupiah(entry.running_balance_idr)}
                          </td>

                          {/* Kolom Lunas (Status) - Persis seperti Excel Gambar 1 */}
                          <td className="p-3 text-center border-l border-slate-100">
                            {!isDebit ? (
                              <span className="text-slate-400">—</span>
                            ) : isFullySettled ? (
                              <span className="inline-flex items-center text-cyan-800 font-semibold text-[10px] bg-cyan-50 px-2.5 py-0.5 rounded-md border border-cyan-200 shadow-2xs">
                                Lunas
                              </span>
                            ) : (
                              <span className="inline-flex items-center text-rose-700 font-semibold text-[10px] bg-rose-50 px-2.5 py-0.5 rounded-md border border-rose-200 shadow-2xs">
                                Belum
                              </span>
                            )}
                          </td>

                          {/* Kolom Sisa - Persis seperti Excel Gambar 1 */}
                          <td className="p-3 text-right font-mono tabular-nums">
                            {!isDebit || isFullySettled ? (
                              <span className="text-slate-400">—</span>
                            ) : (
                              <span className="font-bold text-rose-700">{formatRupiah(entry.remaining_idr)}</span>
                            )}
                          </td>

                          {/* Kolom Umur Piutang - Persis seperti Excel Gambar 1 */}
                          <td className="p-3 pr-6 text-right font-mono text-[11px] whitespace-nowrap">
                            {!isDebit || isFullySettled ? (
                              <span className="text-slate-400">—</span>
                            ) : entry.age_days == null ? (
                              <span className="text-purple-700">Tgl tidak diketahui</span>
                            ) : entry.age_days < 0 ? (
                              <span className="text-purple-700">Masa depan</span>
                            ) : entry.age_days > 60 ? (
                              <span className="font-bold text-rose-700">60 &gt; Hari</span>
                            ) : entry.age_days > 30 ? (
                              <span className="font-semibold text-amber-800">{entry.age_days} hari</span>
                            ) : (
                              <span className="text-slate-700 font-medium">{entry.age_days} hari</span>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}

            {/* Pagination Controls */}
            {(query.cursors.length > 0 || state.data.meta.next_cursor) && (
              <div className="flex items-center justify-between p-4 border-t border-slate-100 bg-slate-50/50">
                <span className="text-xs text-slate-500">Navigasi Riwayat Transaksi</span>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={!query.cursors.length}
                    onClick={() => setQuery({ ...query, cursors: query.cursors.slice(0, -1) })}
                    className="text-xs"
                  >
                    Sebelumnya
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={!state.data.meta.next_cursor}
                    onClick={() => {
                      const next = state.data.meta.next_cursor;
                      if (next) setQuery({ ...query, cursors: [...query.cursors, next] });
                    }}
                    className="text-xs"
                  >
                    Berikutnya
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
