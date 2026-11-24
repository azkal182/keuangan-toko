import { useCallback, useRef, useState, type FormEvent } from "react";
import { AlertCircle, RotateCcw, Link2, CheckCircle2 } from "lucide-react";
import { api, failure, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Dialog } from "./components/ui/dialog";
import { formatRupiah } from "./money";

export function ReverseTransactionAction({
  id,
  csrf,
  onChanged,
}: {
  id: string;
  csrf: string;
  onChanged: () => void;
}) {
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);
  const [open, setOpen] = useState(false);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    setPending(true);
    try {
      readResult(
        await api.POST("/api/v1/transactions/{id}/reversal", {
          params: {
            path: { id },
            header: { "X-CSRF-Token": csrf, "Idempotency-Key": crypto.randomUUID() },
          },
          body: { reason },
        }),
      );
      setOpen(false);
      setReason("");
      onChanged();
    } catch (value) {
      setError(failure(value).message);
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={() => {
          setError(undefined);
          setOpen(true);
        }}
        className="gap-1.5 text-xs text-rose-700 border-rose-200 hover:bg-rose-50"
      >
        <RotateCcw className="h-3 w-3" />
        <span>Koreksi / Pembalikan</span>
      </Button>

      <Dialog open={open} onOpenChange={setOpen} title="Balik Transaksi Historis (Reversal)">
        <form className="space-y-4" onSubmit={submit}>
          <div className="p-3.5 rounded-xl border border-rose-200 bg-rose-50 text-xs text-rose-900">
            <strong>Peringatan Pemilik:</strong> Pembalikan transaksi bersifat permanen dan akan langsung menyesuaikan saldo buku piutang pelanggan dengan mencatat entri kontra audit.
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="rev-reason">
              Alasan Pembalikan Transaksi <span className="text-rose-600">*</span>
            </label>
            <Input
              id="rev-reason"
              required
              maxLength={1000}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="contoh: Salah input nominal faktur saat penjualan…"
              className="text-xs"
            />
          </div>

          {error && <ErrorNotice message={error} />}

          <div className="flex items-center justify-end gap-2 pt-2">
            <Button variant="outline" size="sm" disabled={pending} onClick={() => setOpen(false)}>
              Batal
            </Button>
            <Button type="submit" variant="destructive" size="sm" disabled={pending}>
              {pending ? "Membalik Transaksi…" : "Konfirmasi Pembalikan"}
            </Button>
          </div>
        </form>
      </Dialog>
    </>
  );
}

export function ReplacementLinkAction({ id, csrf }: { id: string; csrf: string }) {
  const [replacement, setReplacement] = useState("");
  const [reason, setReason] = useState("");
  const [notice, setNotice] = useState<string>();
  const [error, setError] = useState<string>();

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setNotice(undefined);
    setError(undefined);
    try {
      const result = await api.POST("/api/v1/transactions/{id}/replacement", {
        params: {
          path: { id },
          header: { "X-CSRF-Token": csrf, "Idempotency-Key": crypto.randomUUID() },
        },
        body: { replacement_transaction_id: replacement, reason },
      });
      readResult(result);
      setNotice("Transaksi pengganti berhasil ditautkan.");
    } catch (value) {
      setError(failure(value).message);
    }
  }

  return (
    <form className="space-y-4 p-4 rounded-xl border border-slate-200 bg-slate-50/50" onSubmit={submit}>
      <h4 className="text-xs font-bold text-slate-900 flex items-center gap-1.5">
        <Link2 className="h-3.5 w-3.5 text-slate-500" />
        <span>Tautkan Transaksi Pengganti</span>
      </h4>

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="rep-id">
          ID Transaksi Pengganti
        </label>
        <Input
          id="rep-id"
          required
          value={replacement}
          onChange={(e) => setReplacement(e.target.value)}
          placeholder="UUID transaksi baru…"
          className="text-xs font-mono"
        />
      </div>

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="rep-reason">
          Alasan Penautan
        </label>
        <Input
          id="rep-reason"
          required
          maxLength={1000}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="contoh: Pengganti faktur yang dibatalkan…"
          className="text-xs"
        />
      </div>

      {notice && (
        <div className="flex items-center gap-2 p-3 rounded-lg border border-emerald-200 bg-emerald-50 text-emerald-900 text-xs font-medium">
          <CheckCircle2 className="h-4 w-4 text-emerald-600 shrink-0" />
          <span>{notice}</span>
        </div>
      )}

      {error && <ErrorNotice message={error} />}

      <Button type="submit" variant="outline" size="sm" className="text-xs">
        Simpan Tautan
      </Button>
    </form>
  );
}

export function ReallocatePaymentAction({
  payment,
  csrf,
  onChanged,
}: {
  payment: Schema["PaymentDetail"];
  csrf: string;
  onChanged: () => void;
}) {
  const [reason, setReason] = useState("");
  const [debitId, setDebitId] = useState("");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers/{id}/ledger", {
          params: { path: { id: payment.customer_id }, query: { limit: 100 } },
          signal,
        }),
      ),
    [payment.customer_id],
  );

  const ledger = useResource(load);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    setPending(true);
    try {
      const allocations = debitId && amount ? [{ debit_entry_id: debitId, amount_idr: amount }] : [];
      readResult(
        await api.POST("/api/v1/payments/{id}/reallocation", {
          params: {
            path: { id: payment.id },
            header: { "X-CSRF-Token": csrf, "Idempotency-Key": crypto.randomUUID() },
          },
          body: { expected_version: payment.version, allocations, reason },
        }),
      );
      onChanged();
    } catch (value) {
      setError(failure(value).message);
    } finally {
      setPending(false);
    }
  }

  return (
    <form className="space-y-4 p-4 rounded-xl border border-slate-200 bg-slate-50/50" onSubmit={submit}>
      <h4 className="text-xs font-bold text-slate-900">Atur Ulang Alokasi Pembayaran</h4>
      <p className="text-xs text-slate-500">
        Alokasi aktif pembayaran ini akan dibalik lebih dulu, lalu nominal baru dialokasikan ke piutang pilihan.
      </p>

      {ledger.state.status === "loading" && <p className="text-xs text-slate-500">Memuat piutang terbuka…</p>}
      {ledger.state.status === "error" && <ErrorNotice message={ledger.state.error.message} retry={ledger.reload} />}

      {ledger.state.status === "ready" && (
        <div className="space-y-3">
          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="realloc-debit">
              Piutang Tujuan Baru
            </label>
            <select
              id="realloc-debit"
              value={debitId}
              onChange={(e) => setDebitId(e.target.value)}
              className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 text-xs text-slate-900"
            >
              <option value="">Jadikan Saldo Kredit Pelanggan (Tidak Dialokasikan)</option>
              {ledger.state.data.data.entries
                .filter((entry) => BigInt(entry.debit_idr) > 0n && entry.allocation_status !== "allocated")
                .map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.effective_date ?? "Tgl Tidak Diketahui"} • {entry.entry_type} • {formatRupiah(entry.debit_idr)}
                  </option>
                ))}
            </select>
          </div>

          {debitId && (
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="realloc-amt">
                Nominal Alokasi Baru (Rp)
              </label>
              <Input
                id="realloc-amt"
                required
                inputMode="numeric"
                pattern="[0-9]+"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="text-xs font-mono"
              />
            </div>
          )}
        </div>
      )}

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="realloc-reason">
          Alasan Perubahan Alokasi
        </label>
        <Input
          id="realloc-reason"
          required
          maxLength={1000}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="contoh: Permintaan penyesuaian alokasi ke faktur lama…"
          className="text-xs"
        />
      </div>

      {error && <ErrorNotice message={error} />}

      <Button
        type="submit"
        variant="outline"
        size="sm"
        disabled={pending || ledger.state.status !== "ready"}
        className="text-xs font-semibold"
      >
        {pending ? "Menyimpan Alokasi…" : "Balik Alokasi Lama & Simpan"}
      </Button>
    </form>
  );
}

export function LegacyCreditApplication({
  customerId,
  paymentSourceId,
  availableAmount,
  csrf,
  onChanged,
}: {
  customerId: string;
  paymentSourceId: string;
  availableAmount: string;
  csrf: string;
  onChanged: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [debitId, setDebitId] = useState("");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);
  const attempt = useRef<{ body: string; key: string } | undefined>(undefined);

  const load = useCallback(
    async (signal: AbortSignal) => {
      const entries: Schema["LedgerEntry"][] = [];
      let cursor: string | undefined;
      do {
        const page = readResult(
          await api.GET("/api/v1/customers/{id}/ledger", {
            params: { path: { id: customerId }, query: { ...(cursor ? { cursor } : {}), limit: 100 } },
            signal,
          }),
        );
        entries.push(...page.data.entries);
        cursor = page.meta.next_cursor ?? undefined;
      } while (cursor && !signal.aborted);
      return entries;
    },
    [customerId],
  );

  const ledger = useResource(load);
  const debits =
    ledger.state.status === "ready"
      ? ledger.state.data.filter(
          (entry) =>
            BigInt(entry.debit_idr) > 0n && !entry.is_reversed && entry.allocation_status !== "allocated",
        )
      : [];

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (!debitId || !amount || BigInt(amount) <= 0n || BigInt(amount) > BigInt(availableAmount)) {
      setError("Pilih transaksi piutang dan masukkan nominal dalam batas saldo kredit yang tersedia.");
      return;
    }
    const body = { payment_id: paymentSourceId, debit_entry_id: debitId, amount_idr: amount, reason };
    const fingerprint = JSON.stringify(body);
    if (attempt.current?.body !== fingerprint)
      attempt.current = { body: fingerprint, key: crypto.randomUUID() };
    setPending(true);
    try {
      readResult(
        await api.POST("/api/v1/customers/{id}/credit-applications", {
          params: { path: { id: customerId }, header: { "X-CSRF-Token": csrf, "Idempotency-Key": attempt.current.key } },
          body,
        }),
      );
      attempt.current = undefined;
      setOpen(false);
      setAmount("");
      setReason("");
      onChanged();
    } catch (value) {
      setError(failure(value).message);
    } finally {
      setPending(false);
    }
  }

  if (!open) {
    return (
      <Button variant="outline" size="sm" onClick={() => setOpen(true)} className="text-xs">
        Terapkan Kredit Historis
      </Button>
    );
  }

  return (
    <form className="space-y-4 p-4 rounded-xl border border-slate-200 bg-white" onSubmit={submit}>
      <h4 className="text-xs font-bold text-slate-900">Terapkan Saldo Kredit Historis</h4>

      {ledger.state.status === "loading" && <p className="text-xs text-slate-500">Memuat piutang terbuka…</p>}
      {ledger.state.status === "error" && <ErrorNotice message={ledger.state.error.message} retry={ledger.reload} />}

      {ledger.state.status === "ready" && (
        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-slate-700" htmlFor="leg-debit">
            Piutang Terbuka Tujuan
          </label>
          <select
            id="leg-debit"
            value={debitId}
            onChange={(e) => setDebitId(e.target.value)}
            required
            className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 text-xs text-slate-900"
          >
            <option value="">-- Pilih transaksi piutang --</option>
            {debits.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.effective_date ?? "Tgl Tidak Diketahui"} • {entry.entry_type} • {formatRupiah((BigInt(entry.debit_idr) - BigInt(entry.allocated_idr)).toString())}
              </option>
            ))}
          </select>
        </div>
      )}

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="leg-amount">
          Nominal (Maks. {formatRupiah(availableAmount)})
        </label>
        <Input
          id="leg-amount"
          inputMode="numeric"
          pattern="[0-9]+"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          required
          className="text-xs font-mono font-bold"
        />
      </div>

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="leg-reason">
          Alasan Alokasi
        </label>
        <Input
          id="leg-reason"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={1000}
          required
          className="text-xs"
        />
      </div>

      {error && <ErrorNotice message={error} />}

      <div className="flex items-center justify-end gap-2 pt-2">
        <Button variant="outline" size="sm" onClick={() => setOpen(false)} disabled={pending}>
          Batal
        </Button>
        <Button
          type="submit"
          size="sm"
          disabled={pending || ledger.state.status !== "ready" || !debits.length}
          className="font-semibold"
        >
          {pending ? "Menerapkan…" : "Terapkan Kredit"}
        </Button>
      </div>
    </form>
  );
}
