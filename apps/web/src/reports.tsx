import { useCallback, useState } from "react";
import { api, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { formatRupiah } from "./ledger";

const buckets: { key: Schema["AgingBucket"]; label: string }[] = [
  { key: "days_0_15", label: "0–15 hari" }, { key: "days_16_30", label: "16–30 hari" },
  { key: "days_31_45", label: "31–45 hari" }, { key: "days_46_60", label: "46–60 hari" },
  { key: "days_over_60", label: "Lebih dari 60 hari" }, { key: "unknown_date", label: "Tanggal tidak diketahui" },
  { key: "future_date", label: "Tanggal masa depan" },
];

export function ReceivablesAging() {
  const today = new Intl.DateTimeFormat("en-CA", { timeZone: "Asia/Jakarta", year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date());
  const [asOf, setAsOf] = useState(today);
  const [applied, setApplied] = useState(today);
  const [bucket, setBucket] = useState<Schema["AgingBucket"]>();
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/reports/receivables-aging", {
    params: { query: { as_of: applied } }, signal,
  })), [applied]);
  const { state, reload } = useResource(load);
  return <section className="panel stack" aria-labelledby="aging-title">
    <h2 id="aging-title">Umur piutang</h2>
    <form className="date-form" onSubmit={(event) => { event.preventDefault(); setApplied(asOf); setBucket(undefined); }}>
      <label>Tanggal acuan<Input type="date" value={asOf} max={today} onChange={(event) => setAsOf(event.target.value)} /></label>
      <Button type="submit" variant="outline">Terapkan</Button>
    </form>
    {state.status === "loading" && <p role="status">Memuat ringkasan piutang…</p>}
    {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
    {state.status === "ready" && <>
      <p className="muted">Per tanggal {state.data.data.as_of}. Nilai mencakup piutang terbuka; tanggal pengecualian tetap masuk total.</p>
      <p className="ledger-total"><span>Total piutang terbuka</span><strong>{formatRupiah(state.data.data.total_open_idr)}</strong></p>
      <div className="table-scroll" tabIndex={0} role="region" aria-label="Ringkasan umur piutang">
        <table><caption className="sr-only">Ringkasan umur piutang</caption><thead><tr><th>Kelompok umur</th><th className="number">Jumlah</th></tr></thead>
          <tbody>{buckets.map(({ key, label }) => <tr key={key}><th scope="row"><button type="button" className="sale-link" aria-pressed={bucket === key} onClick={() => setBucket(key)}>{label}</button></th><td className="number">{formatRupiah(state.data.data[key])}</td></tr>)}</tbody>
        </table>
      </div>
      {bucket && <AgingCustomerList key={`${applied}-${bucket}`} asOf={applied} bucket={bucket} label={buckets.find((item) => item.key === bucket)?.label ?? "Rincian"} onClose={() => setBucket(undefined)} />}
    </>}
  </section>;
}

function AgingCustomerList({ asOf, bucket, label, onClose }: { asOf: string; bucket: Schema["AgingBucket"]; label: string; onClose: () => void }) {
  const [cursor, setCursor] = useState<string>();
  const [previous, setPrevious] = useState<string[]>([]);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/reports/receivables-aging/customers", {
    params: { query: { as_of: asOf, bucket, cursor } }, signal,
  })), [asOf, bucket, cursor]);
  const { state, reload } = useResource(load);
  return <section className="panel stack" aria-label={`Rincian ${label}`}><div className="workspace-heading"><h3>Rincian: {label}</h3><Button variant="plain" onClick={onClose}>Tutup</Button></div>
    {state.status === "loading" && <p role="status">Memuat rincian…</p>}
    {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
    {state.status === "ready" && (state.data.data.length === 0 ? <p className="muted">Tidak ada piutang pada kelompok ini.</p> : <>
      <div className="table-scroll" tabIndex={0} role="region" aria-label="Pelanggan dalam kelompok umur"><table><thead><tr><th>Pelanggan</th><th className="number">Piutang terbuka</th></tr></thead><tbody>
        {state.data.data.map((customer) => <tr key={customer.customer_id}><th scope="row">{customer.customer_name}</th><td className="number">{formatRupiah(customer.amount_idr)}</td></tr>)}
      </tbody></table></div>
      {(previous.length > 0 || state.data.meta.next_cursor) && <div className="pagination"><Button variant="outline" disabled={!previous.length} onClick={() => { const stack = previous.slice(0,-1); setPrevious(stack); setCursor(stack.at(-1)); }}>Sebelumnya</Button><Button variant="outline" disabled={!state.data.meta.next_cursor} onClick={() => { const next = state.data.meta.next_cursor; if (next) { setPrevious([...previous, next]); setCursor(next); } }}>Berikutnya</Button></div>}
    </>)}
  </section>;
}
