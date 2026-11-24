import { useCallback, useState } from "react";
import { api, readResult, useResource } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { LegacyCreditApplication, ReverseTransactionAction } from "./corrections";

const rupiah = new Intl.NumberFormat("id-ID", { style: "currency", currency: "IDR", maximumFractionDigits: 0 });
const businessDate = new Intl.DateTimeFormat("id-ID", { day: "numeric", month: "short", year: "numeric", timeZone: "UTC" });

export function formatRupiah(amount: string) {
  return rupiah.format(BigInt(amount));
}

const entryLabels: Record<string, string> = {
  sale: "Penjualan kredit", credit_sale: "Penjualan kredit",
  payment: "Pembayaran", legacy_debit: "Debit historis", legacy_payment: "Pembayaran historis",
  reversal: "Pembalikan",
};

export function Ledger({ id, openSale, csrf, owner, onChanged }: {
  id: string; openSale: (id: string) => void; csrf: string; owner: boolean; onChanged: () => void;
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
            <div className="table-scroll" tabIndex={0} role="region" aria-label="Riwayat transaksi">
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
                        {owner && entry.entry_type.startsWith("legacy_") && !entry.is_reversed && <div className="stack">
                          {entry.entry_type === "legacy_payment" && BigInt(entry.credit_idr) > BigInt(entry.allocated_idr) && <LegacyCreditApplication customerId={id} paymentSourceId={entry.source_id} availableAmount={(BigInt(entry.credit_idr) - BigInt(entry.allocated_idr)).toString()} csrf={csrf} onChanged={() => { reload(); onChanged(); }} />}
                          <ReverseTransactionAction id={entry.id} csrf={csrf} onChanged={() => { reload(); onChanged(); }} />
                        </div>}
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
