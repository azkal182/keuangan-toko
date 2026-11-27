import { useCallback, useRef, useState, type FormEvent } from "react";
import { DollarSign, CheckCircle2, ArrowRight } from "lucide-react";
import { api, failure, readResult, useResource, ApiFailure, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { MoneyDisplay } from "./components/money-display";
import { formatRupiah } from "./money";

type Customer = Schema["Customer"];

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

export function PaymentsPanel({
  customer,
  csrf,
  canWrite,
  onChanged,
}: {
  customer: Customer;
  csrf: string;
  canWrite: boolean;
  onChanged: () => void;
}) {
  const [creating, setCreating] = useState(false);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers/{id}/payments", {
          params: { path: { id: customer.id }, query: { limit: 100 } },
          signal,
        }),
      ),
    [customer.id],
  );

  const { state, reload } = useResource(load);

  return (
    <Card className="border-slate-200/90 shadow-2xs">
      <CardHeader className="border-b border-slate-100 pb-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <DollarSign className="h-4 w-4 text-emerald-800" />
              <CardTitle className="text-base font-bold text-slate-900">
                Pencatatan Pembayaran & Alokasi FIFO
              </CardTitle>
            </div>
            <p className="text-xs text-slate-500 mt-0.5">
              Pembayaran tunai atau transfer bank yang otomatis dialokasikan ke piutang terlama.
            </p>
          </div>

          {canWrite && !creating && (
            <Button
              onClick={() => setCreating(true)}
              size="sm"
              className="gap-1.5 text-xs font-semibold shadow-sm"
            >
              <span>+ Catat Pembayaran</span>
            </Button>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-6 space-y-6">
        {creating && (
          <div className="p-5 rounded-xl border border-emerald-200 bg-emerald-50/30">
            <PaymentForm
              customer={customer}
              csrf={csrf}
              onCancel={() => setCreating(false)}
              onSaved={() => {
                setCreating(false);
                reload();
                onChanged();
              }}
            />
          </div>
        )}

        {state.status === "loading" && (
          <div className="py-8 text-center text-sm text-slate-500">Memuat riwayat pembayaran…</div>
        )}

        {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}

        {state.status === "ready" && state.data.data.length === 0 && (
          <div className="text-center py-6 text-slate-500 text-xs">
            Belum ada riwayat pembayaran untuk pelanggan ini.
          </div>
        )}

        {state.status === "ready" && state.data.data.length > 0 && (
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-slate-700 uppercase tracking-wider">
              Daftar Riwayat Pembayaran ({state.data.data.length})
            </h4>

            <div className="divide-y divide-slate-100 rounded-xl border border-slate-200 overflow-hidden bg-white shadow-2xs">
              {state.data.data.map((payment) => {
                const unallocated = BigInt(payment.unallocated_idr);
                const hasCredit = unallocated > 0n;
                const allocated = BigInt(payment.amount_idr) - unallocated;

                return (
                  <div key={payment.id} className="p-4 hover:bg-slate-50/70 transition-colors">
                    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                      <div className="space-y-1">
                        <div className="flex items-center gap-3">
                          <MoneyDisplay amount={payment.amount_idr} size="lg" variant="credit" />
                          <Badge variant="outline" className="font-mono text-xs">
                            {payment.method === "cash" ? "Tunai" : "Transfer Bank"}
                          </Badge>
                          <Badge variant={payment.status === "posted" ? "success" : "secondary"}>
                            {payment.status}
                          </Badge>
                        </div>

                        <div className="flex flex-wrap items-center gap-2 text-xs text-slate-500">
                          <span className="font-mono">{payment.received_date}</span>
                          <span>•</span>
                          <span>Dialokasikan: <strong className="text-slate-800 font-mono">{formatRupiah(allocated.toString())}</strong></span>
                          {payment.reference && (
                            <>
                              <span>•</span>
                              <span>Ref: {payment.reference}</span>
                            </>
                          )}
                        </div>

                        {payment.note && (
                          <p className="text-xs text-slate-600 bg-slate-50 px-2 py-1 rounded">
                            {payment.note}
                          </p>
                        )}
                      </div>

                      {/* Unallocated surplus badge & action */}
                      <div className="flex flex-col sm:items-end gap-2">
                        {hasCredit && (
                          <div className="flex items-center gap-2">
                            <span className="text-xs text-slate-500">Saldo Lebih (Kredit):</span>
                            <MoneyDisplay amount={payment.unallocated_idr} size="md" variant="credit" />
                          </div>
                        )}

                        {canWrite && hasCredit && (
                          <CreditApplication
                            customer={customer}
                            payment={payment}
                            csrf={csrf}
                            onApplied={() => {
                              reload();
                              onChanged();
                            }}
                          />
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function PaymentForm({
  customer,
  csrf,
  onCancel,
  onSaved,
}: {
  customer: Customer;
  csrf: string;
  onCancel: () => void;
  onSaved: () => void;
}) {
  const [review, setReview] = useState<Schema["CreatePayment"]>();
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  const idempotency = useRef<string | undefined>(undefined);

  function prepare(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    const data: Schema["CreatePayment"] = {
      customer_id: customer.id,
      amount_idr: String(values.get("amount_idr")),
      received_date: String(values.get("received_date") ?? "").trim(),
      method: String(values.get("method")) as Schema["PaymentMethod"],
      reference: String(values.get("reference")) || null,
      note: String(values.get("note")) || null,
    };
    if (!data.received_date) {
      setError(new ApiFailure("Tanggal diterima wajib diisi.", 422));
      return;
    }
    const today = todayJakarta();
    if (data.received_date > today) {
      setError(new ApiFailure("Tanggal diterima tidak boleh melebihi tanggal hari ini.", 422));
      return;
    }
    if (!/^\d+$/.test(data.amount_idr) || BigInt(data.amount_idr) <= 0n) {
      setError(new ApiFailure("Nominal pembayaran harus berupa bilangan rupiah bulat lebih dari nol.", 422));
      return;
    }
    setError(undefined);
    setReview(data);
    idempotency.current = crypto.randomUUID();
  }

  async function submit() {
    if (!review) return;
    setPending(true);
    setError(undefined);
    try {
      readResult(
        await api.POST("/api/v1/payments", {
          params: {
            header: {
              "X-CSRF-Token": csrf,
              "Idempotency-Key": idempotency.current ?? crypto.randomUUID(),
            },
          },
          body: review,
        }),
      );
      onSaved();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="border-b border-emerald-200 pb-2">
        <h3 className="text-sm font-bold text-emerald-950">
          {!review ? "Formulir Pencatatan Pembayaran" : "Konfirmasi Pembayaran & Alokasi"}
        </h3>
      </div>

      {!review ? (
        <form className="space-y-4" onSubmit={prepare}>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="pay-amount">
                Nominal Pembayaran (Rp) <span className="text-rose-600">*</span>
              </label>
              <Input
                id="pay-amount"
                name="amount_idr"
                inputMode="numeric"
                pattern="[0-9]+"
                required
                placeholder="contoh: 500000"
                className="text-xs font-mono font-bold"
              />
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="pay-date">
                Tanggal Diterima <span className="text-rose-600">*</span>
              </label>
              <Input
                id="pay-date"
                name="received_date"
                type="date"
                max={todayJakarta()}
                defaultValue=""
                required
                className="text-xs"
              />
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="pay-method">
                Metode Pembayaran
              </label>
              <select
                id="pay-method"
                name="method"
                defaultValue="cash"
                className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
              >
                <option value="cash">Tunai (Cash)</option>
                <option value="bank_transfer">Transfer Bank</option>
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="pay-ref">
                Referensi Bukti / No. Transfer (opsional)
              </label>
              <Input
                id="pay-ref"
                name="reference"
                maxLength={256}
                placeholder="contoh: TRX-BCA-987654"
                className="text-xs"
              />
            </div>

            <div className="sm:col-span-2 space-y-1.5">
              <label className="text-xs font-semibold text-slate-700" htmlFor="pay-note">
                Catatan Tambahan (opsional)
              </label>
              <textarea
                id="pay-note"
                name="note"
                rows={2}
                maxLength={2000}
                placeholder="Catatan dari kasir atau pembawa uang…"
                className="w-full rounded-lg border border-slate-300 p-2 text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
              />
            </div>
          </div>

          {error && <ErrorNotice message={error.message} />}

          <div className="flex items-center justify-end gap-2 pt-2">
            <Button variant="outline" size="sm" onClick={onCancel}>
              Batal
            </Button>
            <Button type="submit" size="sm" className="font-semibold">
              Periksa Pembayaran
            </Button>
          </div>
        </form>
      ) : (
        <div className="space-y-4">
          <div className="p-4 rounded-xl border border-slate-200 bg-white space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs text-slate-500">Nominal Diserahkan:</span>
              <MoneyDisplay amount={review.amount_idr} size="lg" variant="credit" />
            </div>
            <div className="flex items-center justify-between text-xs">
              <span className="text-slate-500">Metode & Tanggal:</span>
              <span className="font-medium text-slate-800">
                {review.method === "cash" ? "Tunai" : "Transfer Bank"} • {review.received_date}
              </span>
            </div>
          </div>

          <p className="text-xs text-slate-600 bg-emerald-50 p-3 rounded-lg border border-emerald-200">
            Sistem secara otomatis mengalokasikan pembayaran ini ke piutang terlama (FIFO). Jika terdapat kelebihan pembayaran, sisa uang akan masuk sebagai <strong>Saldo Kredit Pelanggan</strong>.
          </p>

          {error && <ErrorNotice message={error.message} />}

          <div className="flex items-center justify-end gap-2 pt-2">
            <Button
              variant="outline"
              size="sm"
              disabled={pending}
              onClick={() => {
                setReview(undefined);
                idempotency.current = undefined;
              }}
            >
              Kembali Ubah
            </Button>
            <Button
              size="sm"
              disabled={pending}
              onClick={submit}
              className="gap-1.5 font-semibold"
            >
              <CheckCircle2 className="h-4 w-4" />
              <span>{pending ? "Mencatat ke Buku…" : "Konfirmasi & Catat Pembayaran"}</span>
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

function CreditApplication({
  customer,
  payment,
  csrf,
  onApplied,
}: {
  customer: Customer;
  payment: Schema["PaymentDetail"];
  csrf: string;
  onApplied: () => void;
}) {
  const [open, setOpen] = useState(false);

  if (!open) {
    return (
      <Button
        variant="outline"
        size="sm"
        onClick={() => setOpen(true)}
        className="gap-1.5 text-xs text-emerald-800 border-emerald-300 hover:bg-emerald-50"
      >
        <ArrowRight className="h-3.5 w-3.5" />
        <span>Terapkan Saldo Kredit ke Piutang</span>
      </Button>
    );
  }

  return (
    <div className="mt-3 p-4 rounded-xl border border-emerald-200 bg-white shadow-sm">
      <CreditApplicationForm
        customer={customer}
        payment={payment}
        csrf={csrf}
        onCancel={() => setOpen(false)}
        onApplied={() => {
          setOpen(false);
          onApplied();
        }}
      />
    </div>
  );
}

function CreditApplicationForm({
  customer,
  payment,
  csrf,
  onCancel,
  onApplied,
}: {
  customer: Customer;
  payment: Schema["PaymentDetail"];
  csrf: string;
  onCancel: () => void;
  onApplied: () => void;
}) {
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  const attempt = useRef<{ fingerprint: string; key: string } | undefined>(undefined);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers/{id}/ledger", {
          params: { path: { id: customer.id }, query: { limit: 100 } },
          signal,
        }),
      ),
    [customer.id],
  );

  const { state, reload } = useResource(load);
  const debits =
    state.status === "ready"
      ? state.data.data.entries.filter(
          (entry) =>
            BigInt(entry.debit_idr) > 0n &&
            (entry.allocation_status === "unallocated" ||
              entry.allocation_status === "partially_allocated"),
        )
      : [];

  async function apply(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    const body = {
      payment_id: payment.id,
      debit_entry_id: String(values.get("debit_entry_id")),
      amount_idr: String(values.get("amount_idr")),
      reason: String(values.get("reason")),
    };
    const fingerprint = JSON.stringify(body);
    if (attempt.current?.fingerprint !== fingerprint) {
      attempt.current = { fingerprint, key: crypto.randomUUID() };
    }
    setPending(true);
    setError(undefined);
    try {
      readResult(
        await api.POST("/api/v1/customers/{id}/credit-applications", {
          params: {
            path: { id: customer.id },
            header: { "X-CSRF-Token": csrf, "Idempotency-Key": attempt.current.key },
          },
          body,
        }),
      );
      attempt.current = undefined;
      onApplied();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <form className="space-y-4" onSubmit={apply}>
      <div className="border-b border-slate-100 pb-2">
        <h4 className="text-xs font-bold text-slate-900">
          Terapkan Saldo Kredit ke Transaksi Piutang
        </h4>
        <p className="text-xs text-slate-500">
          Saldo tersedia: <strong className="text-emerald-700 font-mono">{formatRupiah(payment.unallocated_idr)}</strong>
        </p>
      </div>

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="debit-select">
          Pilih Transaksi Piutang Tujuan
        </label>
        <select
          id="debit-select"
          name="debit_entry_id"
          required
          disabled={state.status !== "ready"}
          className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-700"
        >
          <option value="">-- Pilih salah satu transaksi piutang --</option>
          {debits.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.effective_date ?? "Tgl Tidak Diketahui"} • {formatRupiah(entry.debit_idr)} • {entry.description ?? entry.entry_type}
            </option>
          ))}
        </select>
      </div>

      {state.status === "loading" && <p className="text-xs text-slate-500">Memuat transaksi piutang…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && debits.length === 0 && (
        <p className="text-xs text-slate-500 bg-slate-50 p-2.5 rounded border border-slate-200">
          Tidak ada transaksi piutang terbuka yang memerlukan alokasi.
        </p>
      )}

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="apply-amount">
          Nominal Dialokasikan (Rp)
        </label>
        <Input
          id="apply-amount"
          name="amount_idr"
          inputMode="numeric"
          pattern="[0-9]+"
          max={payment.unallocated_idr}
          required
          placeholder={payment.unallocated_idr}
          className="text-xs font-mono font-bold"
        />
      </div>

      <div className="space-y-1.5">
        <label className="text-xs font-semibold text-slate-700" htmlFor="apply-reason">
          Alasan Alokasi Kredit
        </label>
        <textarea
          id="apply-reason"
          name="reason"
          rows={2}
          maxLength={1000}
          required
          placeholder="contoh: Alokasi deposit pelanggan ke faktur pembelian baru…"
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
          size="sm"
          disabled={pending || state.status !== "ready" || !debits.length}
          className="font-semibold"
        >
          {pending ? "Menerapkan…" : "Konfirmasi Alokasi"}
        </Button>
      </div>
    </form>
  );
}
