import { useCallback, useRef, useState, type FormEvent } from "react";
import { Plus, Trash2, CheckCircle2, ShoppingBag, X, Send } from "lucide-react";
import { api, failure, readResult, useResource, type ApiFailure, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { MoneyDisplay } from "./components/money-display";
import { formatRupiah } from "./money";

type Customer = Schema["Customer"];
type Sale = Schema["Sale"];
type SaleInput = Schema["CreateSale"];

function businessToday() {
  const parts = Object.fromEntries(
    new Intl.DateTimeFormat("en", {
      timeZone: "Asia/Jakarta",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    })
      .formatToParts(new Date())
      .map(({ type, value }) => [type, value]),
  );
  return `${parts.year}-${parts.month}-${parts.day}`;
}

type Props = {
  customer: Customer;
  csrf: string;
  canWrite: boolean;
  owner: boolean;
  saleId?: string;
  onOpen: (id?: string) => void;
  onPosted: () => void;
};

export function SalesPanel(props: Props) {
  const [creating, setCreating] = useState(false);

  if (props.saleId) return <SaleReview key={props.saleId} {...props} id={props.saleId} />;
  if (!props.canWrite || !props.customer.is_active) return null;

  return (
    <Card className="border-slate-200/90 shadow-2xs">
      <CardHeader className="border-b border-slate-100 pb-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <CardTitle className="text-base font-bold text-slate-900 flex items-center gap-2">
              <ShoppingBag className="h-4 w-4 text-emerald-800" />
              <span>Penjualan Kredit (Piutang Baru)</span>
            </CardTitle>
            <p className="text-xs text-slate-500 mt-0.5">
              Catat faktur penjualan kredit dengan rincian item barang.
            </p>
          </div>

          {!creating && (
            <Button
              onClick={() => setCreating(true)}
              size="sm"
              className="gap-1.5 text-xs font-semibold shadow-sm"
            >
              <Plus className="h-4 w-4" />
              <span>Buat Draft Penjualan</span>
            </Button>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-6">
        {creating ? (
          <SaleForm
            {...props}
            onCancel={() => setCreating(false)}
            onSaved={(sale) => {
              setCreating(false);
              props.onOpen(sale.id);
            }}
          />
        ) : (
          <div className="text-center py-6 text-slate-500 text-xs">
            Tekan tombol <strong>"Buat Draft Penjualan"</strong> untuk mencatat faktur penjualan kredit baru.
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function SaleReview({
  id,
  customer,
  csrf,
  canWrite,
  onOpen,
  onPosted,
  owner,
}: Props & { id: string }) {
  const [editing, setEditing] = useState(false);
  const [posting, setPosting] = useState(false);
  const [error, setError] = useState<ApiFailure>();
  const attempt = useRef<{ version: number; key: string } | null>(null);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/credit-sales/{id}", {
          params: { path: { id } },
          signal,
        }),
      ).data,
    [id],
  );

  const { state, reload } = useResource(load);
  const sale = state.status === "ready" && state.data.customer_id === customer.id ? state.data : undefined;
  const draft = sale?.status === "draft";

  async function postSale(sale: Sale) {
    setPosting(true);
    setError(undefined);
    if (attempt.current?.version !== sale.version) {
      attempt.current = { version: sale.version, key: crypto.randomUUID() };
    }
    try {
      readResult(
        await api.POST("/api/v1/credit-sales/{id}/post", {
          params: { path: { id }, header: { "X-CSRF-Token": csrf, "Idempotency-Key": attempt.current.key } },
          body: { expected_version: sale.version },
        }),
      );
      onPosted();
    } catch (error) {
      setError(failure(error));
    } finally {
      setPosting(false);
    }
  }

  return (
    <Card className="border-slate-200/90 shadow-2xs">
      <CardHeader className="border-b border-slate-100 pb-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <ShoppingBag className="h-4 w-4 text-emerald-800" />
              <CardTitle className="text-base font-bold text-slate-900">
                Faktur Penjualan Kredit: {sale?.reference_no ?? id.slice(0, 8)}
              </CardTitle>
            </div>
            <p className="text-xs text-slate-500 mt-0.5">
              Tinjau rincian barang, total nilai faktur, dan status posting ke buku piutang.
            </p>
          </div>

          <Button
            variant="outline"
            size="sm"
            disabled={posting}
            onClick={() => onOpen(undefined)}
            className="gap-1.5 text-xs text-slate-600"
          >
            <X className="h-3.5 w-3.5" />
            <span>Tutup Rincian</span>
          </Button>
        </div>
      </CardHeader>

      <CardContent className="p-6 space-y-6">
        {state.status === "loading" && (
          <div className="py-8 text-center text-sm text-slate-500">Memuat data penjualan…</div>
        )}

        {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}

        {state.status === "ready" && state.data.customer_id !== customer.id && (
          <ErrorNotice message="Penjualan ini tercatat pada pelanggan lain." />
        )}

        {sale && (
          <>
            {/* Meta Tags */}
            <div className="flex flex-wrap items-center gap-3 p-4 rounded-xl bg-slate-50 border border-slate-200 text-xs">
              <div className="flex items-center gap-2">
                <span className="text-slate-500">Status:</span>
                <Badge
                  variant={draft ? "warning" : sale.status === "posted" ? "success" : "destructive"}
                  className="font-semibold text-xs"
                >
                  {draft ? "Draft (Belum Diposting)" : sale.status === "posted" ? "Diposting ke Ledger" : "Dibalik (Reversed)"}
                </Badge>
              </div>

              <div className="flex items-center gap-2 pl-3 border-l border-slate-300">
                <span className="text-slate-500">Tanggal Transaksi:</span>
                <span className="font-mono font-semibold text-slate-800">{sale.effective_date}</span>
              </div>

              {sale.reference_no && (
                <div className="flex items-center gap-2 pl-3 border-l border-slate-300">
                  <span className="text-slate-500">No. Faktur:</span>
                  <span className="font-mono font-medium text-slate-700">{sale.reference_no}</span>
                </div>
              )}
            </div>

            {sale.backdate_reason && (
              <p className="text-xs text-slate-600 bg-amber-50 p-3 rounded-lg border border-amber-200">
                <strong>Alasan Tanggal Lampau (Backdate):</strong> {sale.backdate_reason}
              </p>
            )}

            {editing ? (
              <SaleForm
                key={sale.version}
                customer={customer}
                csrf={csrf}
                owner={owner}
                sale={sale}
                onCancel={() => setEditing(false)}
                onReload={reload}
                onSaved={() => {
                  setEditing(false);
                  setError(undefined);
                  reload();
                }}
              />
            ) : (
              <div className="space-y-4">
                {/* Line Item Table */}
                <div className="rounded-xl border border-slate-200 overflow-hidden shadow-2xs">
                  <table className="w-full text-left text-xs border-collapse">
                    <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 font-semibold uppercase text-[11px]">
                      <tr>
                        <th className="p-3 pl-4">No</th>
                        <th className="p-3 min-w-[12rem]">Uraian Barang</th>
                        <th className="p-3 text-right">Kuantitas</th>
                        <th className="p-3 text-right">Harga Satuan (Rp)</th>
                        <th className="p-3 pr-4 text-right">Total Item (Rp)</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-100">
                      {sale.items.map((item, index) => (
                        <tr key={item.id} className="hover:bg-slate-50/50">
                          <td className="p-3 pl-4 text-slate-400 font-mono">{index + 1}</td>
                          <td className="p-3 font-medium text-slate-800">{item.description}</td>
                          <td className="p-3 text-right font-mono text-slate-700">{item.quantity}</td>
                          <td className="p-3 text-right font-mono text-slate-700">
                            {formatRupiah(item.unit_price_idr)}
                          </td>
                          <td className="p-3 pr-4 text-right font-mono font-bold text-slate-900">
                            {formatRupiah(item.line_total_idr)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                    <tfoot className="bg-slate-50 border-t border-slate-200">
                      <tr>
                        <td colSpan={4} className="p-3 pl-4 text-right font-semibold text-slate-700">
                          Total Nilai Penjualan Kredit:
                        </td>
                        <td className="p-3 pr-4 text-right">
                          <MoneyDisplay amount={sale.total_idr} size="lg" variant="danger" />
                        </td>
                      </tr>
                    </tfoot>
                  </table>
                </div>

                {sale.status === "posted" && (
                  <div className="flex items-center gap-2 p-3.5 rounded-xl border border-emerald-200 bg-emerald-50 text-emerald-900 text-xs font-medium">
                    <CheckCircle2 className="h-4 w-4 text-emerald-600 shrink-0" />
                    <span>Faktur penjualan ini telah resmi diposting ke buku piutang pelanggan.</span>
                  </div>
                )}

                {error && <ErrorNotice message={error.message} />}

                {/* Actions for draft sale */}
                {draft && canWrite && customer.is_active && (
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pt-4 border-t border-slate-200 bg-amber-50/30 p-4 rounded-xl border border-amber-200/60">
                    <p className="text-xs text-amber-900">
                      Pastikan item dan total sudah sesuai. Menekan tombol posting akan menambah saldo piutang pelanggan di buku piutang secara permanen.
                    </p>
                    <div className="flex items-center gap-2 shrink-0">
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={posting}
                        onClick={() => {
                          setEditing(true);
                          setError(undefined);
                        }}
                        className="text-xs"
                      >
                        Ubah Draft
                      </Button>
                      <Button
                        size="sm"
                        disabled={posting}
                        onClick={() => postSale(sale)}
                        className="gap-1.5 text-xs font-semibold"
                      >
                        <Send className="h-3.5 w-3.5" />
                        <span>{posting ? "Memposting…" : "Posting ke Ledger"}</span>
                      </Button>
                    </div>
                  </div>
                )}
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}

function SaleForm({
  customer,
  csrf,
  owner,
  sale,
  onSaved,
  onCancel,
  onReload,
}: {
  customer: Customer;
  csrf: string;
  owner: boolean;
  sale?: Sale;
  onSaved: (sale: Sale) => void;
  onCancel: () => void;
  onReload?: () => void;
}) {
  const [date, setDate] = useState(() => sale?.effective_date ?? businessToday());
  const [items, setItems] = useState<(Schema["SaleItemInput"] & { key: string })[]>(() =>
    sale?.items.map(({ id, description, quantity, unit_price_idr }) => ({
      key: id,
      description,
      quantity,
      unit_price_idr,
    })) ?? [{ key: crypto.randomUUID(), description: "", quantity: "1", unit_price_idr: "" }],
  );
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  const attempt = useRef<{ body: string; key: string } | null>(null);
  const today = businessToday();

  function updateItem(key: string, field: keyof Schema["SaleItemInput"], value: string) {
    setItems((current) =>
      current.map((item) => (item.key === key ? { ...item, [field]: value } : item)),
    );
  }

  // Calculate live preview total
  const computedGrandTotal = items.reduce((sum, item) => {
    const qty = parseFloat(item.quantity) || 0;
    const price = BigInt(item.unit_price_idr || "0");
    return sum + (price * BigInt(Math.round(qty)));
  }, 0n);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const input: SaleInput = {
      customer_id: customer.id,
      effective_date: date,
      reference_no: String(data.get("reference") ?? "") || null,
      backdate_reason: date < today ? (String(data.get("reason") ?? "").trim() || null) : null,
      items: items.map(({ description, quantity, unit_price_idr }) => ({
        description,
        quantity,
        unit_price_idr,
      })),
    };

    setError(undefined);
    setPending(true);
    try {
      const header = { "X-CSRF-Token": csrf };
      if (sale) {
        const result = await api.PATCH("/api/v1/credit-sales/{id}", {
          params: { path: { id: sale.id }, header },
          body: { ...input, expected_version: sale.version },
        });
        onSaved(readResult(result).data);
      } else {
        const body = JSON.stringify(input);
        if (attempt.current?.body !== body) attempt.current = { body, key: crypto.randomUUID() };
        const result = await api.POST("/api/v1/credit-sales", {
          params: { header: { ...header, "Idempotency-Key": attempt.current.key } },
          body: input,
        });
        onSaved(readResult(result).data);
      }
    } catch (error) {
      setError(failure(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <form className="space-y-6" onSubmit={save}>
      {/* Top Invoice Metadata */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 p-4 rounded-xl bg-slate-50 border border-slate-200">
        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-slate-700" htmlFor="sale-date">
            Tanggal Usaha Transaksi <span className="text-rose-600">*</span>
          </label>
          <Input
            id="sale-date"
            type="date"
            required
            value={date}
            max={today}
            aria-invalid={!!error?.fields.effective_date}
            onChange={(e) => setDate(e.target.value)}
            className="text-xs"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-slate-700" htmlFor="sale-ref">
            Nomor Faktur / Referensi (opsional)
          </label>
          <Input
            id="sale-ref"
            name="reference"
            maxLength={256}
            defaultValue={sale?.reference_no ?? ""}
            placeholder="contoh: FKT-2026/11/001"
            className="text-xs"
          />
        </div>

        {date && date < today && (
          <div className="sm:col-span-2 space-y-1.5">
            <label className="text-xs font-semibold text-amber-900" htmlFor="sale-backdate-reason">
              Catatan / Alasan Tanggal Lampau (opsional)
            </label>
            <textarea
              id="sale-backdate-reason"
              name="reason"
              rows={2}
              maxLength={1000}
              defaultValue={sale?.backdate_reason ?? ""}
              placeholder="Catatan tanggal lampau untuk riwayat pembukuan (opsional)…"
              className="w-full rounded-lg border border-amber-300 bg-amber-50/50 p-2 text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
            />
          </div>
        )}
      </div>

      {/* Spreadsheet-Like Line-Item Table */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <h4 className="text-xs font-bold text-slate-900 uppercase tracking-wider">
            Rincian Item Barang
          </h4>
          <span className="text-xs text-slate-500">{items.length} item</span>
        </div>

        <div className="rounded-xl border border-slate-200 overflow-hidden shadow-2xs">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 font-semibold uppercase text-[11px]">
              <tr>
                <th className="p-3 pl-4 w-12">No</th>
                <th className="p-3 min-w-[14rem]">
                  Uraian Barang <span className="text-rose-600">*</span>
                </th>
                <th className="p-3 w-28 text-right">
                  Kuantitas <span className="text-rose-600">*</span>
                </th>
                <th className="p-3 w-36 text-right">
                  Harga Satuan (Rp) <span className="text-rose-600">*</span>
                </th>
                <th className="p-3 w-36 text-right font-bold text-slate-900">
                  Subtotal
                </th>
                <th className="p-3 pr-4 w-12 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {items.map((item, index) => {
                const descError = error?.fields[`items.${index}.description`];
                const qtyError = error?.fields[`items.${index}.quantity`];
                const priceError = error?.fields[`items.${index}.unit_price_idr`];

                const qtyVal = parseFloat(item.quantity) || 0;
                const priceVal = BigInt(item.unit_price_idr || "0");
                const rowTotal = priceVal * BigInt(Math.round(qtyVal));

                return (
                  <tr key={item.key} className="hover:bg-slate-50/50">
                    <td className="p-3 pl-4 text-slate-400 font-mono">{index + 1}</td>
                    <td className="p-2">
                      <Input
                        required
                        value={item.description}
                        placeholder="contoh: Beras Ramos 50kg"
                        maxLength={1000}
                        aria-invalid={!!descError}
                        onChange={(e) => updateItem(item.key, "description", e.target.value)}
                        className="h-8 text-xs"
                      />
                      {descError && <p className="text-[10px] text-rose-600 mt-0.5">{descError}</p>}
                    </td>
                    <td className="p-2">
                      <Input
                        required
                        type="number"
                        step="any"
                        min="0.001"
                        value={item.quantity}
                        aria-invalid={!!qtyError}
                        onChange={(e) => updateItem(item.key, "quantity", e.target.value)}
                        className="h-8 text-xs font-mono text-right"
                      />
                      {qtyError && <p className="text-[10px] text-rose-600 mt-0.5">{qtyError}</p>}
                    </td>
                    <td className="p-2">
                      <Input
                        required
                        inputMode="numeric"
                        pattern="[0-9]+"
                        value={item.unit_price_idr}
                        placeholder="0"
                        aria-invalid={!!priceError}
                        onChange={(e) => updateItem(item.key, "unit_price_idr", e.target.value)}
                        className="h-8 text-xs font-mono text-right"
                      />
                      {priceError && <p className="text-[10px] text-rose-600 mt-0.5">{priceError}</p>}
                    </td>
                    <td className="p-3 text-right font-mono font-bold text-slate-900">
                      {formatRupiah(rowTotal.toString())}
                    </td>
                    <td className="p-2 pr-4 text-center">
                      {items.length > 1 && (
                        <button
                          type="button"
                          title="Hapus baris item"
                          onClick={() => setItems((curr) => curr.filter((r) => r.key !== item.key))}
                          className="p-1 rounded text-slate-400 hover:text-rose-600 hover:bg-rose-50 transition-colors"
                        >
                          <Trash2 className="h-4 w-4" />
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
            <tfoot className="bg-slate-50 border-t border-slate-200">
              <tr>
                <td colSpan={4} className="p-3 pl-4 text-right font-bold text-slate-700">
                  Estimasi Total Faktur:
                </td>
                <td className="p-3 text-right font-mono text-base font-bold text-rose-700">
                  {formatRupiah(computedGrandTotal.toString())}
                </td>
                <td />
              </tr>
            </tfoot>
          </table>
        </div>
      </div>

      {error && <ErrorNotice message={error.message} />}

      {/* Action Footer */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pt-4 border-t border-slate-200">
        <Button
          variant="outline"
          size="sm"
          onClick={() =>
            setItems((curr) => [
              ...curr,
              { key: crypto.randomUUID(), description: "", quantity: "1", unit_price_idr: "" },
            ])
          }
          className="gap-1.5 text-xs"
        >
          <Plus className="h-4 w-4" />
          <span>Tambah Baris Item</span>
        </Button>

        <div className="flex items-center gap-2 justify-end">
          <Button variant="outline" size="sm" onClick={onCancel} disabled={pending}>
            Batal
          </Button>
          {error?.status === 409 && onReload && (
            <Button variant="outline" size="sm" onClick={onReload}>
              Muat Ulang Penjualan
            </Button>
          )}
          <Button type="submit" size="sm" disabled={pending} className="gap-1.5 font-semibold">
            {pending ? "Menyimpan Draft…" : "Simpan & Tinjau Draft"}
          </Button>
        </div>
      </div>
    </form>
  );
}
