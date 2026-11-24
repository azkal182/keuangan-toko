import { useCallback, useState } from "react";
import { api, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { formatRupiah } from "./money";
const businessDate = new Intl.DateTimeFormat("id-ID", { day: "numeric", month: "short", year: "numeric", timeZone: "UTC" });

const entryLabels: Record<string, string> = {
  sale: "Penjualan kredit", credit_sale: "Penjualan kredit",
  payment: "Pembayaran", legacy_debit: "Debit historis", legacy_payment: "Pembayaran historis",
  reversal: "Pembalikan",
};

export function Ledger({ id, openSale }: {
  id: string; openSale: (id: string) => void;
}) {
  const [query, setQuery] = useState<{ from?: string; to?: string; cursors: string[] }>({ cursors: [] });
  const cursor = query.cursors.at(-1);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers/{id}/ledger", {
    params: { path: { id }, query: { from: query.from, to: query.to, cursor } }, signal,
  })), [id, query, cursor]);
  const { state, reload } = useResource(load);

  return (
    <section className="panel stack ledger" aria-labelledby="ledger-title">
      <h2 id="ledger-title">Buku piutang</h2>
      <form className="date-form" onSubmit={(event) => {
        event.preventDefault();
        const values = new FormData(event.currentTarget);
        setQuery({ from: String(values.get("from")) || undefined, to: String(values.get("to")) || undefined, cursors: [] });
      }}>
        <label>Dari tanggal<Input name="from" type="date" /></label>
        <label>Sampai tanggal<Input name="to" type="date" /></label>
        <Button type="submit" variant="outline">Terapkan</Button>
        <Button type="reset" variant="plain" onClick={() => setQuery({ cursors: [] })}>Reset</Button>
      </form>
      {state.status === "loading" && <p role="status">Memuat buku piutang…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && (
        <>
          <p className="ledger-total"><span>Saldo bersih buku piutang</span><strong>{formatRupiah(state.data.data.summary.ledger_balance_idr)}</strong></p>
          {state.data.data.entries.length === 0 ? (
            <p className="empty-state">Belum ada transaksi untuk periode ini.</p>
          ) : (
            <>
            <div className="table-scroll ledger-desktop" tabIndex={0} role="region" aria-label="Riwayat transaksi">
              <table>
                <caption className="sr-only">Riwayat transaksi buku piutang</caption>
                <thead>
                  <tr><th scope="col" rowSpan={2}>Tanggal</th><th scope="col" rowSpan={2}>Uraian</th><th scope="col" rowSpan={2} className="number">Jumlah</th><th scope="col" rowSpan={2} className="number">Harga</th><th scope="col" rowSpan={2} className="number">Debet</th><th scope="col" rowSpan={2} className="number">Kredit</th><th scope="col" rowSpan={2} className="number">Saldo</th><th scope="colgroup" colSpan={3}>Keterangan</th></tr>
                  <tr><th scope="col" className="number">Lunas</th><th scope="col" className="number">Sisa</th><th scope="col" className="number">Umur Piutang</th></tr>
                </thead>
                <tbody>
                  {state.data.data.entries.map((entry) => (
                    <tr key={entry.id}>
                      <td>{entry.effective_date ? businessDate.format(new Date(entry.effective_date)) : "Tidak diketahui"}</td>
                      <td>
                        {entry.entry_type === "credit_sale" && entry.description ? <button type="button" className="sale-link" onClick={() => openSale(entry.source_id)}>{entry.description}</button> : <strong>{entry.description ?? entryLabels[entry.entry_type] ?? entry.entry_type}</strong>}
                        {entry.is_reversed && <span className="table-note">Dibalik</span>}
                      </td>
                      <td className="number">{entry.quantity ?? "—"}</td>
                      <td className="number">{entry.unit_price_idr == null ? "—" : formatRupiah(entry.unit_price_idr)}</td>
                      <td className="number">{entry.debit_idr === "0" ? "—" : formatRupiah(entry.debit_idr)}</td>
                      <td className="number">{entry.credit_idr === "0" ? "—" : formatRupiah(entry.credit_idr)}</td>
                      <td className="number">{formatRupiah(entry.running_balance_idr)}</td>
                      <td className="number">{BigInt(entry.debit_idr) === 0n ? "—" : formatRupiah(entry.allocated_idr)}</td>
                      <td className="number">{BigInt(entry.debit_idr) === 0n ? "—" : formatRupiah(entry.remaining_idr)}</td>
                      <td className="number">{entry.age_days == null ? (entry.debit_idr === "0" ? "—" : "Tanggal tidak diketahui") : entry.age_days < 0 ? "Tanggal masa depan" : `${entry.age_days} hari`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="ledger-mobile-cards" aria-label="Riwayat transaksi">
              {state.data.data.entries.map((entry) => <LedgerMobileCard key={entry.id} entry={entry} openSale={openSale} />)}
            </div>
            </>
          )}
          {(query.cursors.length > 0 || state.data.meta.next_cursor) && (
            <div className="pagination">
              <Button variant="outline" disabled={!query.cursors.length} onClick={() => setQuery({ ...query, cursors: query.cursors.slice(0, -1) })}>Sebelumnya</Button>
              <Button variant="outline" disabled={!state.data.meta.next_cursor} onClick={() => {
                const next = state.data.meta.next_cursor;
                if (next) setQuery({ ...query, cursors: [...query.cursors, next] });
              }}>Berikutnya</Button>
            </div>
          )}
        </>
      )}
    </section>
  );
}

function LedgerMobileCard({ entry, openSale }: {
  entry: Schema["LedgerEntry"];
  openSale: (id: string) => void;
}) {
  const description = entry.description ?? entryLabels[entry.entry_type] ?? entry.entry_type;
  return <article className="ledger-mobile-card">
    <div className="ledger-mobile-heading">
      <span>{entry.effective_date ? businessDate.format(new Date(entry.effective_date)) : "Tanggal tidak diketahui"}</span>
      {entry.age_days != null && <span>{entry.age_days < 0 ? "Tanggal masa depan" : `${entry.age_days} hari`}</span>}
    </div>
    <strong>{entry.entry_type === "credit_sale" && entry.description
      ? <button type="button" className="sale-link" onClick={() => openSale(entry.source_id)}>{description}</button>
      : description}</strong>
    {entry.is_reversed && <span className="badge">Dibalik</span>}
    <dl className="ledger-mobile-values">
      <div><dt>Jumlah</dt><dd>{entry.quantity ?? "—"}</dd></div>
      <div><dt>Harga</dt><dd>{entry.unit_price_idr == null ? "—" : formatRupiah(entry.unit_price_idr)}</dd></div>
      <div><dt>Debet</dt><dd>{entry.debit_idr === "0" ? "—" : formatRupiah(entry.debit_idr)}</dd></div>
      <div><dt>Kredit</dt><dd>{entry.credit_idr === "0" ? "—" : formatRupiah(entry.credit_idr)}</dd></div>
      <div><dt>Saldo</dt><dd>{formatRupiah(entry.running_balance_idr)}</dd></div>
      <div><dt>Lunas</dt><dd>{BigInt(entry.debit_idr) === 0n ? "—" : formatRupiah(entry.allocated_idr)}</dd></div>
      <div><dt>Sisa</dt><dd>{BigInt(entry.debit_idr) === 0n ? "—" : formatRupiah(entry.remaining_idr)}</dd></div>
      <div><dt>Umur piutang</dt><dd>{entry.age_days == null ? (entry.debit_idr === "0" ? "—" : "Tanggal tidak diketahui") : entry.age_days < 0 ? "Tanggal masa depan" : `${entry.age_days} hari`}</dd></div>
    </dl>
  </article>;
}
