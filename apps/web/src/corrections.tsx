import { useCallback, useRef, useState, type FormEvent } from "react";
import { api, failure, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { formatRupiah } from "./ledger";
import { Dialog } from "./components/ui/dialog";

export function ReverseTransactionAction({ id, csrf, onChanged }: { id: string; csrf: string; onChanged: () => void }) {
  const [reason, setReason] = useState(""); const [error, setError] = useState<string>(); const [pending, setPending] = useState(false); const [open, setOpen] = useState(false);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setError(undefined); setPending(true);
    try { readResult(await api.POST("/api/v1/transactions/{id}/reversal", { params: { path: { id }, header: { "X-CSRF-Token": csrf, "Idempotency-Key": crypto.randomUUID() } }, body: { reason } })); setOpen(false); setReason(""); onChanged(); }
    catch (value) { setError(failure(value).message); } finally { setPending(false); }
  }
  return <><Button type="button" variant="outline" onClick={() => { setError(undefined); setOpen(true); }}>Koreksi transaksi</Button><Dialog open={open} onOpenChange={setOpen} title="Balik transaksi historis">
    <form className="stack correction-form" onSubmit={submit}>
      <p className="muted">Pembalikan bersifat permanen dan akan mengubah saldo piutang pelanggan.</p>
      <label>Alasan pembalikan<Input required maxLength={1000} value={reason} onChange={(event) => setReason(event.target.value)} /></label>
      {error && <ErrorNotice message={error} />}
      <div className="actions"><Button type="submit" variant="outline" disabled={pending}>{pending ? "Membalik transaksi…" : "Konfirmasi pembalikan"}</Button><Button type="button" variant="plain" disabled={pending} onClick={() => setOpen(false)}>Batal</Button></div>
    </form>
  </Dialog></>;
}

export function ReplacementLinkAction({ id, csrf }: { id: string; csrf: string }) {
  const [replacement, setReplacement] = useState(""); const [reason, setReason] = useState(""); const [notice, setNotice] = useState<string>(); const [error, setError] = useState<string>();
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setNotice(undefined); setError(undefined);
    try { const result=await api.POST("/api/v1/transactions/{id}/replacement",{params:{path:{id},header:{"X-CSRF-Token":csrf,"Idempotency-Key":crypto.randomUUID()}},body:{replacement_transaction_id:replacement,reason}});readResult(result);setNotice("Transaksi pengganti tertaut ke koreksi."); }
    catch(value){setError(failure(value).message);}
  }
  return <form className="stack correction-form" onSubmit={submit}>
    <h3>Tautkan transaksi pengganti</h3><label>ID transaksi pengganti<Input required value={replacement} onChange={(event)=>setReplacement(event.target.value)} /></label>
    <label>Alasan tautan<Input required maxLength={1000} value={reason} onChange={(event)=>setReason(event.target.value)} /></label>
    {notice&&<p role="status" className="notice-success">{notice}</p>}{error&&<ErrorNotice message={error}/>}
    <Button type="submit" variant="outline">Simpan tautan</Button>
  </form>;
}

export function ReallocatePaymentAction({ payment, csrf, onChanged }: { payment: Schema["PaymentDetail"]; csrf: string; onChanged: () => void }) {
  const [reason,setReason]=useState("");const [debitId,setDebitId]=useState("");const [amount,setAmount]=useState("");const [error,setError]=useState<string>();const [pending,setPending]=useState(false);
  const load=useCallback(async(signal:AbortSignal)=>readResult(await api.GET("/api/v1/customers/{id}/ledger",{params:{path:{id:payment.customer_id},query:{limit:100}},signal})),[payment.customer_id]);
  const ledger=useResource(load);
  async function submit(event:FormEvent<HTMLFormElement>){event.preventDefault();setError(undefined);setPending(true);try{
    const allocations=debitId&&amount?[{debit_entry_id:debitId,amount_idr:amount}]:[];
    readResult(await api.POST("/api/v1/payments/{id}/reallocation",{params:{path:{id:payment.id},header:{"X-CSRF-Token":csrf,"Idempotency-Key":crypto.randomUUID()}},body:{expected_version:payment.version,allocations,reason}}));onChanged();
  }catch(value){setError(failure(value).message);}finally{setPending(false);}}
  return <form className="stack correction-form" onSubmit={submit}>
    <h3>Atur ulang alokasi</h3><p className="muted">Alokasi aktif pembayaran ini akan dibalik lebih dulu. Nominal baru tidak boleh melebihi saldo piutang.</p>
    {ledger.state.status === "loading" && <p role="status">Memuat piutang terbuka…</p>}
    {ledger.state.status === "error" && <ErrorNotice message={ledger.state.error.message} retry={ledger.reload} />}
    {ledger.state.status === "ready" && <>
      <label>Piutang tujuan<select className="input" value={debitId} onChange={(event)=>setDebitId(event.target.value)}><option value="">Jadikan kredit pelanggan</option>{ledger.state.data.data.entries.filter((entry)=>BigInt(entry.debit_idr)>0n&&entry.allocation_status!=="allocated").map((entry)=><option key={entry.id} value={entry.id}>{entry.effective_date??"Tanggal tidak diketahui"} · {entry.entry_type} · {formatRupiah(entry.debit_idr)}</option>)}</select></label>
      {debitId&&<label>Nominal alokasi baru<Input required inputMode="numeric" pattern="[0-9]+" value={amount} onChange={(event)=>setAmount(event.target.value)} /></label>}
    </>}
    <label>Alasan perubahan<Input required maxLength={1000} value={reason} onChange={(event)=>setReason(event.target.value)} /></label>
    {error&&<ErrorNotice message={error}/>}
    <Button type="submit" variant="outline" disabled={pending||ledger.state.status!=="ready"}>{pending?"Menyimpan alokasi…":"Balik alokasi lama dan simpan perubahan"}</Button>
  </form>;
}

export function LegacyCreditApplication({ customerId, paymentSourceId, availableAmount, csrf, onChanged }: {
  customerId: string; paymentSourceId: string; availableAmount: string; csrf: string; onChanged: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [debitId, setDebitId] = useState("");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);
  const attempt = useRef<{ body: string; key: string } | undefined>(undefined);
  const load = useCallback(async (signal: AbortSignal) => {
    const entries: Schema["LedgerEntry"][] = [];
    let cursor: string | undefined;
    do {
      const page = readResult(await api.GET("/api/v1/customers/{id}/ledger", {
        params: { path: { id: customerId }, query: { ...(cursor ? { cursor } : {}), limit: 100 } }, signal,
      }));
      entries.push(...page.data.entries);
      cursor = page.meta.next_cursor ?? undefined;
    } while (cursor && !signal.aborted);
    return entries;
  }, [customerId]);
  const ledger = useResource(load);
  const debits = ledger.state.status === "ready" ? ledger.state.data.filter((entry) =>
    BigInt(entry.debit_idr) > 0n && !entry.is_reversed && entry.allocation_status !== "allocated",
  ) : [];

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setError(undefined);
    if (!debitId || !amount || BigInt(amount) <= 0n || BigInt(amount) > BigInt(availableAmount)) {
      setError("Pilih piutang dan nominal dalam batas saldo kredit yang tersedia."); return;
    }
    const body = { payment_id: paymentSourceId, debit_entry_id: debitId, amount_idr: amount, reason };
    const fingerprint = JSON.stringify(body);
    if (attempt.current?.body !== fingerprint) attempt.current = { body: fingerprint, key: crypto.randomUUID() };
    setPending(true);
    try {
      readResult(await api.POST("/api/v1/customers/{id}/credit-applications", {
        params: { path: { id: customerId }, header: { "X-CSRF-Token": csrf, "Idempotency-Key": attempt.current.key } }, body,
      }));
      attempt.current = undefined; setOpen(false); setAmount(""); setReason(""); onChanged();
    } catch (value) { setError(failure(value).message); }
    finally { setPending(false); }
  }

  if (!open) return <Button variant="outline" onClick={() => setOpen(true)}>Terapkan kredit historis</Button>;
  return <form className="stack correction-form" onSubmit={submit}>
    {ledger.state.status === "loading" && <p role="status">Memuat piutang terbuka…</p>}
    {ledger.state.status === "error" && <ErrorNotice message={ledger.state.error.message} retry={ledger.reload} />}
    {ledger.state.status === "ready" && <label>Piutang tujuan<select className="input" value={debitId} onChange={(event) => setDebitId(event.target.value)} required>
      <option value="">Pilih piutang</option>{debits.map((entry) => <option key={entry.id} value={entry.id}>{entry.effective_date ?? "Tanggal tidak diketahui"} · {entry.entry_type} · {formatRupiah((BigInt(entry.debit_idr) - BigInt(entry.allocated_idr)).toString())}</option>)}
    </select></label>}
    <label>Nominal (maks. {formatRupiah(availableAmount)})<Input inputMode="numeric" pattern="[0-9]+" value={amount} onChange={(event) => setAmount(event.target.value)} required /></label>
    <label>Alasan<Input value={reason} onChange={(event) => setReason(event.target.value)} maxLength={1000} required /></label>
    {error && <ErrorNotice message={error} />}
    <div className="actions"><Button type="submit" disabled={pending || ledger.state.status !== "ready" || !debits.length}>{pending ? "Menerapkan…" : "Terapkan kredit"}</Button><Button type="button" variant="outline" onClick={() => setOpen(false)} disabled={pending}>Batal</Button></div>
  </form>;
}
