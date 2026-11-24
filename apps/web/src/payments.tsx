import { useCallback, useRef, useState, type FormEvent } from "react";
import { api, failure, readResult, useResource, ApiFailure, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { formatRupiah } from "./ledger";
import { ReallocatePaymentAction, ReverseTransactionAction } from "./corrections";

type Customer = Schema["Customer"];

function todayJakarta() {
  const parts = Object.fromEntries(new Intl.DateTimeFormat("en", {
    timeZone: "Asia/Jakarta", year: "numeric", month: "2-digit", day: "2-digit",
  }).formatToParts(new Date()).map(({ type, value }) => [type, value]));
  return `${parts.year}-${parts.month}-${parts.day}`;
}

export function PaymentsPanel({ customer, csrf, canWrite, owner, onChanged }: {
  customer: Customer; csrf: string; canWrite: boolean; owner: boolean; onChanged: () => void;
}) {
  const [creating, setCreating] = useState(false);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers/{id}/payments", {
    params: { path: { id: customer.id }, query: { limit: 100 } }, signal,
  })), [customer.id]);
  const { state, reload } = useResource(load);

  return (
    <section className="panel stack" aria-labelledby="payments-title">
      <div className="detail-heading">
        <h2 id="payments-title">Pembayaran</h2>
        {canWrite && !creating && <Button onClick={() => setCreating(true)}>Catat pembayaran</Button>}
      </div>
      {creating && <PaymentForm customer={customer} csrf={csrf} onCancel={() => setCreating(false)} onSaved={() => {
        setCreating(false);
        reload();
        onChanged();
      }} />}
      {state.status === "loading" && <p role="status">Memuat riwayat pembayaran…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && state.data.data.length === 0 && <p className="muted">Belum ada pembayaran.</p>}
      {state.status === "ready" && state.data.data.length > 0 && (
        <ul className="payment-history">
          {state.data.data.map((payment) => (
            <li key={payment.id} className="payment-row">
              <div className="detail-title">
                <strong>{formatRupiah(payment.amount_idr)}</strong>
                <span>{payment.received_date}</span>
                <span className="muted">{payment.method === "cash" ? "Tunai" : "Transfer bank"}</span>
              </div>
              <p className="muted">Dialokasikan {formatRupiah((BigInt(payment.amount_idr) - BigInt(payment.unallocated_idr)).toString())}
                {payment.reference ? ` · ${payment.reference}` : ""}
              </p>
              {payment.note && <p className="muted">{payment.note}</p>}
              <p className="muted">Status: {payment.status}</p>
              {owner && payment.status === "posted" && <><ReallocatePaymentAction payment={payment} csrf={csrf} onChanged={() => { reload(); onChanged(); }} /><ReverseTransactionAction id={payment.id} csrf={csrf} onChanged={() => { reload(); onChanged(); }} /></>}
              {payment.unallocated_idr !== "0" && <p>Saldo kredit: <strong>{formatRupiah(payment.unallocated_idr)}</strong></p>}
              {canWrite && payment.unallocated_idr !== "0" && (
                <CreditApplication customer={customer} payment={payment} csrf={csrf} onApplied={() => { reload(); onChanged(); }} />
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function PaymentForm({ customer, csrf, onCancel, onSaved }: {
  customer: Customer; csrf: string; onCancel: () => void; onSaved: () => void;
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
      received_date: String(values.get("received_date")),
      method: String(values.get("method")) as Schema["PaymentMethod"],
      reference: String(values.get("reference")) || null,
      note: String(values.get("note")) || null,
    };
    if (!/^\d+$/.test(data.amount_idr) || BigInt(data.amount_idr) <= 0n) {
      setError(new ApiFailure("Nominal harus berupa bilangan rupiah lebih dari nol.", 422));
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
      readResult(await api.POST("/api/v1/payments", {
        params: { header: { "X-CSRF-Token": csrf, "Idempotency-Key": idempotency.current ?? crypto.randomUUID() } },
        body: review,
      }));
      onSaved();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="payment-form stack">
      {!review ? (
        <form className="stack identity-form" onSubmit={prepare}>
          <label>Nominal pembayaran (rupiah)<Input name="amount_idr" inputMode="numeric" pattern="[0-9]+" required /></label>
          <label>Tanggal diterima<Input name="received_date" type="date" max={todayJakarta()} defaultValue={todayJakarta()} required /></label>
          <label>Metode<select className="input" name="method" defaultValue="cash"><option value="cash">Tunai</option><option value="bank_transfer">Transfer bank</option></select></label>
          <label>Referensi (opsional)<Input name="reference" maxLength={256} /></label>
          <label>Catatan (opsional)<textarea className="input" name="note" rows={2} maxLength={2000} /></label>
          {error && <ErrorNotice message={error.message} />}
          <div className="actions"><Button type="submit">Periksa pembayaran</Button><Button type="button" variant="outline" onClick={onCancel}>Batal</Button></div>
        </form>
      ) : (
        <div className="stack" aria-live="polite">
          <h3>Konfirmasi pembayaran</h3>
          <p>{formatRupiah(review.amount_idr)} · {review.received_date} · {review.method === "cash" ? "Tunai" : "Transfer bank"}</p>
          <p className="muted">Pembayaran akan langsung diposting. Sistem mengalokasikan ke piutang terlama terlebih dahulu; sisanya menjadi saldo kredit pelanggan.</p>
          {error && <ErrorNotice message={error.message} />}
          <div className="actions"><Button onClick={submit} disabled={pending}>{pending ? "Mencatat…" : "Konfirmasi dan catat"}</Button>
            <Button variant="outline" onClick={() => { setReview(undefined); idempotency.current = undefined; }} disabled={pending}>Kembali</Button>
          </div>
        </div>
      )}
    </div>
  );
}

function CreditApplication({ customer, payment, csrf, onApplied }: {
  customer: Customer; payment: Schema["PaymentDetail"]; csrf: string; onApplied: () => void;
}) {
  const [open, setOpen] = useState(false);

  if (!open) {
    return <Button variant="outline" onClick={() => setOpen(true)}>Terapkan saldo kredit ke piutang</Button>;
  }

  return <CreditApplicationForm customer={customer} payment={payment} csrf={csrf}
    onCancel={() => setOpen(false)} onApplied={() => { setOpen(false); onApplied(); }} />;
}

function CreditApplicationForm({ customer, payment, csrf, onCancel, onApplied }: {
  customer: Customer; payment: Schema["PaymentDetail"]; csrf: string;
  onCancel: () => void; onApplied: () => void;
}) {
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  const attempt = useRef<{ fingerprint: string; key: string } | undefined>(undefined);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers/{id}/ledger", {
    params: { path: { id: customer.id }, query: { limit: 100 } }, signal,
  })), [customer.id]);
  const { state, reload } = useResource(load);
  const debits = state.status === "ready" ? state.data.data.entries.filter((entry) =>
    BigInt(entry.debit_idr) > 0n &&
    (entry.allocation_status === "unallocated" || entry.allocation_status === "partially_allocated"),
  ) : [];

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
      readResult(await api.POST("/api/v1/customers/{id}/credit-applications", {
        params: { path: { id: customer.id }, header: { "X-CSRF-Token": csrf, "Idempotency-Key": attempt.current.key } },
        body,
      }));
      attempt.current = undefined;
      onApplied();
    } catch (cause) {
      setError(failure(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <form className="stack identity-form" onSubmit={apply}>
      <label>Piutang tujuan<select className="input" name="debit_entry_id" required disabled={state.status !== "ready"}>
        <option value="">Pilih transaksi debit</option>
        {debits.map((entry) => <option key={entry.id} value={entry.id}>{entry.effective_date ?? "Tanggal tidak diketahui"} · {formatRupiah(entry.debit_idr)} · {entry.description ?? entry.entry_type}</option>)}
      </select></label>
      {state.status === "loading" && <p role="status">Memuat piutang…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && debits.length === 0 && <p className="muted">Tidak ada transaksi debit yang dapat dipilih.</p>}
      <label>Nominal rupiah<Input name="amount_idr" inputMode="numeric" pattern="[0-9]+" max={payment.unallocated_idr} required /></label>
      <label>Alasan<textarea className="input" name="reason" rows={2} maxLength={1000} required /></label>
      {error && <ErrorNotice message={error.message} />}
      <div className="actions"><Button type="submit" disabled={pending || state.status !== "ready" || !debits.length}>{pending ? "Menerapkan…" : "Konfirmasi alokasi"}</Button>
        <Button type="button" variant="outline" onClick={onCancel} disabled={pending}>Batal</Button></div>
    </form>
  );
}
