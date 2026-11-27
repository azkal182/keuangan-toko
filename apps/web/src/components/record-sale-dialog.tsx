import { useState, useMemo, type FormEvent } from "react";
import { Plus, Trash2, ShoppingBag, Calendar, CheckCircle2 } from "lucide-react";
import { api, readResult, failure, ApiFailure, type Schema } from "../api";
import { Dialog } from "./ui/dialog";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { formatRupiah } from "../money";
import { MoneyDisplay } from "./money-display";
import { ErrorNotice } from "./presentation";

interface SaleItemRow {
  key: string;
  description: string;
  quantity: string;
  unitPriceIdr: string;
}

function todayJakarta() {
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

export function RecordSaleDialog({
  open,
  onOpenChange,
  customer,
  csrf,
  onSuccess,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  customer: { id: string; name: string };
  csrf: string;
  onSuccess: (message: string) => void;
}) {
  const today = useMemo(() => todayJakarta(), []);
  const [effectiveDate, setEffectiveDate] = useState("");
  const [items, setItems] = useState<SaleItemRow[]>([
    { key: crypto.randomUUID(), description: "", quantity: "1", unitPriceIdr: "" },
  ]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<ApiFailure | undefined>();

  function addItem() {
    setItems((prev) => [
      ...prev,
      { key: crypto.randomUUID(), description: "", quantity: "1", unitPriceIdr: "" },
    ]);
  }

  function removeItem(key: string) {
    if (items.length <= 1) return;
    setItems((prev) => prev.filter((item) => item.key !== key));
  }

  function updateItem(key: string, field: keyof Omit<SaleItemRow, "key">, value: string) {
    setItems((prev) =>
      prev.map((item) => (item.key === key ? { ...item, [field]: value } : item)),
    );
  }

  const computedGrandTotal = useMemo(() => {
    return items.reduce((sum, item) => {
      const qty = parseFloat(item.quantity) || 0;
      const price = parseInt(item.unitPriceIdr.replace(/\D/g, ""), 10) || 0;
      return sum + BigInt(Math.round(qty * price));
    }, 0n);
  }, [items]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);

    if (!effectiveDate) {
      setError(new ApiFailure("Tanggal transaksi wajib diisi."));
      return;
    }

    if (effectiveDate > today) {
      setError(new ApiFailure("Tanggal transaksi tidak boleh melebihi tanggal hari ini."));
      return;
    }

    const validItems = items.filter(
      (item) =>
        item.description.trim() !== "" &&
        (parseInt(item.unitPriceIdr.replace(/\D/g, ""), 10) || 0) > 0,
    );

    if (validItems.length === 0) {
      setError(new ApiFailure("Minimal masukkan 1 barang dengan nama dan harga satuan lebih dari nol."));
      return;
    }

    setSubmitting(true);
    try {
      // Create and post each item as its own entry so it appears cleanly in Buku Piutang per item
      for (const item of validItems) {
        const qtyClean = item.quantity.trim() || "1";
        const priceClean = item.unitPriceIdr.replace(/\D/g, "");

        const draftResult = readResult(
          await api.POST("/api/v1/credit-sales", {
            params: {
              header: {
                "X-CSRF-Token": csrf,
                "Idempotency-Key": crypto.randomUUID(),
              },
            },
            body: {
              customer_id: customer.id,
              effective_date: effectiveDate,
              reference_no: null,
              backdate_reason: null,
              items: [
                {
                  description: item.description.trim(),
                  quantity: qtyClean,
                  unit_price_idr: priceClean,
                },
              ],
            },
          }),
        );

        readResult(
          await api.POST("/api/v1/credit-sales/{id}/post", {
            params: {
              path: { id: draftResult.data.id },
              header: {
                "X-CSRF-Token": csrf,
                "Idempotency-Key": crypto.randomUUID(),
              },
            },
            body: {
              expected_version: draftResult.data.version,
            },
          }),
        );
      }

      // Reset form on success
      setItems([{ key: crypto.randomUUID(), description: "", quantity: "1", unitPriceIdr: "" }]);
      setEffectiveDate("");
      onOpenChange(false);
      onSuccess(
        validItems.length === 1
          ? `Belanja "${validItems[0].description}" berhasil dicatat ke Buku Piutang.`
          : `${validItems.length} item belanja berhasil dicatat ke Buku Piutang.`,
      );
    } catch (err) {
      setError(failure(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!submitting) onOpenChange(next);
      }}
      title={`Catat Belanja / Piutang: ${customer.name}`}
      description="Masukkan barang yang dibeli oleh pelanggan. Setiap barang akan langsung tercatat ke Buku Piutang tanpa proses faktur berbelit."
      maxWidth="max-w-4xl"
    >
      <form onSubmit={handleSubmit} className="space-y-5">
        {error && <ErrorNotice message={error.message} />}

        {/* Transaction Date */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-xl bg-slate-50 border border-slate-200">
          <div className="space-y-0.5">
            <div className="flex items-center gap-2">
              <Calendar className="h-4 w-4 text-emerald-800" />
              <label htmlFor="sale-date" className="text-xs font-bold text-slate-800">
                Tanggal Transaksi: <span className="text-rose-600">*</span>
              </label>
            </div>
            <p className="text-[11px] text-slate-500">
              Wajib diisi dan tidak boleh melebihi tanggal hari ini.
            </p>
          </div>
          <Input
            id="sale-date"
            type="date"
            required
            max={today}
            value={effectiveDate}
            onChange={(e) => setEffectiveDate(e.target.value)}
            className="w-full sm:w-48 h-9 text-xs font-mono bg-white"
          />
        </div>

        {/* Dynamic Items Table */}
        <div className="border border-slate-200 rounded-xl overflow-hidden shadow-2xs">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-slate-100/80 border-b border-slate-200 text-slate-700 font-semibold uppercase text-[11px]">
              <tr>
                <th scope="col" className="p-3 pl-4 w-12 text-center">No</th>
                <th scope="col" className="p-3 min-w-[14rem]">Nama Barang / Uraian</th>
                <th scope="col" className="p-3 text-right w-28">Kuantitas (Qty)</th>
                <th scope="col" className="p-3 text-right w-36">Harga Satuan (Rp)</th>
                <th scope="col" className="p-3 text-right w-36">Subtotal (Rp)</th>
                <th scope="col" className="p-3 pr-4 text-center w-14">Aksi</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 bg-white">
              {items.map((item, index) => {
                const qty = parseFloat(item.quantity) || 0;
                const price = parseInt(item.unitPriceIdr.replace(/\D/g, ""), 10) || 0;
                const subtotal = Math.round(qty * price);

                return (
                  <tr key={item.key} className="hover:bg-slate-50/60 transition-colors">
                    <td className="p-3 pl-4 text-center text-slate-500 font-mono">
                      {index + 1}
                    </td>
                    <td className="p-3">
                      <Input
                        required
                        placeholder="Contoh: Solasih, Lakban Coklat, Dobel Foam…"
                        value={item.description}
                        onChange={(e) => updateItem(item.key, "description", e.target.value)}
                        className="h-8 text-xs"
                      />
                    </td>
                    <td className="p-3">
                      <Input
                        type="number"
                        step="any"
                        min="0.001"
                        required
                        value={item.quantity}
                        onChange={(e) => updateItem(item.key, "quantity", e.target.value)}
                        className="h-8 text-xs text-right font-mono"
                      />
                    </td>
                    <td className="p-3">
                      <Input
                        type="text"
                        inputMode="numeric"
                        required
                        placeholder="0"
                        value={
                          item.unitPriceIdr
                            ? parseInt(item.unitPriceIdr.replace(/\D/g, ""), 10).toLocaleString("id-ID")
                            : ""
                        }
                        onChange={(e) => updateItem(item.key, "unitPriceIdr", e.target.value)}
                        className="h-8 text-xs text-right font-mono"
                      />
                    </td>
                    <td className="p-3 text-right font-mono font-semibold text-slate-800 tabular-nums">
                      {formatRupiah(subtotal.toString())}
                    </td>
                    <td className="p-3 pr-4 text-center">
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        disabled={items.length <= 1}
                        onClick={() => removeItem(item.key)}
                        title="Hapus baris barang ini"
                        className="h-8 w-8 p-0 text-slate-400 hover:text-rose-600 hover:bg-rose-50"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
            <tfoot className="bg-slate-50/90 border-t border-slate-200">
              <tr>
                <td colSpan={6} className="p-3 pl-4">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={addItem}
                    className="gap-1.5 text-xs font-semibold text-emerald-800 border-emerald-300 hover:bg-emerald-50"
                  >
                    <Plus className="h-3.5 w-3.5" />
                    <span>Tambah Baris Barang Lagi</span>
                  </Button>
                </td>
              </tr>
            </tfoot>
          </table>
        </div>

        {/* Total & Submit Controls */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-4 rounded-xl bg-slate-50 border border-slate-200">
          <div className="flex items-center gap-3">
            <span className="text-xs font-semibold text-slate-600 uppercase tracking-wider">
              Total Seluruh Belanja:
            </span>
            <MoneyDisplay amount={computedGrandTotal.toString()} size="lg" variant="danger" />
          </div>

          <div className="flex items-center gap-2.5">
            <Button
              type="button"
              variant="outline"
              disabled={submitting}
              onClick={() => onOpenChange(false)}
              className="text-xs"
            >
              Batal
            </Button>
            <Button
              type="submit"
              disabled={submitting || computedGrandTotal <= 0n}
              className="gap-2 text-xs font-semibold shadow-sm"
            >
              <ShoppingBag className="h-4 w-4" />
              <span>{submitting ? "Menyimpan ke Buku Piutang…" : "Simpan ke Buku Piutang"}</span>
            </Button>
          </div>
        </div>
      </form>
    </Dialog>
  );
}
