import { useCallback, useRef, useState, type FormEvent } from "react";
import { api, failure, readResult, useResource, type ApiFailure, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { formatRupiah } from "./ledger";
import { ReplacementLinkAction, ReverseTransactionAction } from "./corrections";

type Customer = Schema["Customer"];
type Sale = Schema["Sale"];
type SaleInput = Schema["CreateSale"];

function businessToday() {
  const parts = Object.fromEntries(new Intl.DateTimeFormat("en", {
    timeZone: "Asia/Jakarta", year: "numeric", month: "2-digit", day: "2-digit",
  }).formatToParts(new Date()).map(({ type, value }) => [type, value]));
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
    <section className="panel stack">
      <div className="detail-heading">
        <h2>Penjualan kredit</h2>
        {!creating && <Button onClick={() => setCreating(true)}>Buat draft penjualan</Button>}
      </div>
      {creating && <SaleForm {...props} onCancel={() => setCreating(false)} onSaved={(sale) => {
        setCreating(false);
        props.onOpen(sale.id);
      }} />}
    </section>
  );
}

function SaleReview({ id, customer, csrf, owner, canWrite, onOpen, onPosted }: Props & { id: string }) {
  const [editing, setEditing] = useState(false);
  const [posting, setPosting] = useState(false);
  const [error, setError] = useState<ApiFailure>();
  const attempt = useRef<{ version: number; key: string } | null>(null);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/credit-sales/{id}", {
    params: { path: { id } }, signal,
  })).data, [id]);
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
      readResult(await api.POST("/api/v1/credit-sales/{id}/post", {
        params: { path: { id }, header: { "X-CSRF-Token": csrf, "Idempotency-Key": attempt.current.key } },
        body: { expected_version: sale.version },
      }));
      onPosted();
    } catch (error) {
      setError(failure(error));
    } finally {
      setPosting(false);
    }
  }

  return (
    <section className="panel stack" aria-labelledby="sale-review">
      <div className="detail-heading">
        <h2 id="sale-review">Detail penjualan kredit</h2>
        <Button variant="plain" disabled={posting} onClick={() => onOpen()}>Tutup detail penjualan</Button>
      </div>
      {state.status === "loading" && <p role="status">Memuat penjualan…</p>}
      {state.status === "error" && <ErrorNotice message={state.error.message} retry={reload} />}
      {state.status === "ready" && state.data.customer_id !== customer.id && (
        <ErrorNotice message="Penjualan ini tercatat pada pelanggan lain." />
      )}
      {sale && (
        <>
          <div className="detail-title">
            <span className={`badge ${sale.status === "posted" ? "active" : "inactive"}`}>{draft ? "Draft" : sale.status === "posted" ? "Diposting" : "Dibalik"}</span>
            <span>{sale.effective_date}</span>
            {sale.reference_no && <span className="muted">{sale.reference_no}</span>}
          </div>
          {sale.backdate_reason && <p className="muted">Alasan tanggal lampau: {sale.backdate_reason}</p>}
          {editing ? (
            <SaleForm key={sale.version} customer={customer} csrf={csrf} owner={owner} sale={sale}
              onCancel={() => setEditing(false)} onReload={reload}
              onSaved={() => { setEditing(false); setError(undefined); reload(); }} />
          ) : (
            <>
              <div className="table-scroll" tabIndex={0} role="region" aria-label="Item penjualan kredit">
                <table>
                  <caption className="sr-only">Item penjualan kredit</caption>
                  <thead><tr><th scope="col">Uraian</th><th scope="col" className="number">Kuantitas</th><th scope="col" className="number">Harga satuan</th><th scope="col" className="number">Total item</th></tr></thead>
                  <tbody>{sale.items.map((item) => (
                    <tr key={item.id}><td>{item.description}</td><td className="number">{item.quantity}</td>
                      <td className="number">{formatRupiah(item.unit_price_idr)}</td><td className="number">{formatRupiah(item.line_total_idr)}</td></tr>
                  ))}</tbody>
                </table>
              </div>
              <p className="ledger-total"><span>Total penjualan</span><strong>{formatRupiah(sale.total_idr)}</strong></p>
              {sale.status === "posted" && <p className="notice-success" role="status">Penjualan telah tercatat di buku piutang.</p>}
              {owner && sale.status === "posted" && <ReverseTransactionAction id={sale.id} csrf={csrf} onChanged={() => { reload(); onPosted(); }} />}
              {owner && sale.status === "reversed" && <ReplacementLinkAction id={sale.id} csrf={csrf} />}
              {error && <ErrorNotice message={error.message} />}
              {draft && canWrite && customer.is_active && (
                <>
                  <p className="muted">Periksa item dan total sebelum memposting. Posting akan menambah piutang pelanggan.</p>
                  <div className="actions">
                    <Button variant="outline" disabled={posting} onClick={() => { setEditing(true); setError(undefined); }}>Ubah draft</Button>
                    <Button disabled={posting} onClick={() => postSale(sale)}>{posting ? "Memposting…" : "Posting penjualan"}</Button>
                    {error?.status === 409 && <Button variant="outline" onClick={() => { setError(undefined); reload(); }}>Muat ulang penjualan</Button>}
                  </div>
                </>
              )}
            </>
          )}
        </>
      )}
    </section>
  );
}

function SaleForm({ customer, csrf, owner, sale, onSaved, onCancel, onReload }: {
  customer: Customer; csrf: string; owner: boolean; sale?: Sale;
  onSaved: (sale: Sale) => void; onCancel: () => void; onReload?: () => void;
}) {
  const [date, setDate] = useState(() => sale?.effective_date ?? businessToday());
  const [items, setItems] = useState<(Schema["SaleItemInput"] & { key: string })[]>(() => (
    sale?.items.map(({ id, description, quantity, unit_price_idr }) => ({ key: id, description, quantity, unit_price_idr }))
      ?? [{ key: crypto.randomUUID(), description: "", quantity: "1", unit_price_idr: "" }]
  ));
  const [error, setError] = useState<ApiFailure>();
  const [pending, setPending] = useState(false);
  const attempt = useRef<{ body: string; key: string } | null>(null);
  const today = businessToday();

  function updateItem(key: string, field: keyof Schema["SaleItemInput"], value: string) {
    setItems((current) => current.map((item) => item.key === key ? { ...item, [field]: value } : item));
  }

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const input: SaleInput = {
      customer_id: customer.id,
      effective_date: date,
      reference_no: String(data.get("reference") ?? "") || null,
      backdate_reason: owner && date < today ? String(data.get("reason") ?? "") : null,
      items: items.map(({ description, quantity, unit_price_idr }) => ({ description, quantity, unit_price_idr })),
    };
    setError(undefined);
    setPending(true);
    try {
      const header = { "X-CSRF-Token": csrf };
      if (sale) {
        const result = await api.PATCH("/api/v1/credit-sales/{id}", {
          params: { path: { id: sale.id }, header }, body: { ...input, expected_version: sale.version },
        });
        onSaved(readResult(result).data);
      } else {
        const body = JSON.stringify(input);
        if (attempt.current?.body !== body) attempt.current = { body, key: crypto.randomUUID() };
        const result = await api.POST("/api/v1/credit-sales", {
          params: { header: { ...header, "Idempotency-Key": attempt.current.key } }, body: input,
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
    <form className="stack identity-form sale-form" onSubmit={save}>
      <label>Tanggal usaha<Input type="date" required value={date} max={today} min={owner ? undefined : today}
        aria-invalid={!!error?.fields.effective_date} onChange={(event) => setDate(event.target.value)} /></label>
      <label>Referensi penjualan (opsional)<Input name="reference" maxLength={256} defaultValue={sale?.reference_no ?? ""} /></label>
      {owner && date && date < today && (
        <label>Alasan tanggal lampau<textarea name="reason" className="input" rows={2} maxLength={1000} required defaultValue={sale?.backdate_reason ?? ""} /></label>
      )}
      {items.map((item, index) => (
        <fieldset key={item.key} className="sale-item stack">
          <legend>Item {index + 1}</legend>
          {(["description", "quantity", "unit_price_idr"] as const).map((field) => {
            const label = { description: "Uraian", quantity: "Kuantitas", unit_price_idr: "Harga satuan (Rp)" }[field];
            const message = error?.fields[`items.${index}.${field}`];
            return (
              <label key={field}>{label}
                <Input required value={item[field]} inputMode={field === "quantity" ? "decimal" : field === "unit_price_idr" ? "numeric" : undefined}
                  maxLength={field === "description" ? 1000 : 24}
                  aria-invalid={!!message} onChange={(event) => updateItem(item.key, field, event.target.value)} />
                {message && <span className="field-error">{message}</span>}
              </label>
            );
          })}
          {items.length > 1 && <Button variant="plain" onClick={() => setItems((current) => current.filter((row) => row.key !== item.key))}>Hapus item {index + 1}</Button>}
        </fieldset>
      ))}
      {error && <ErrorNotice message={error.message} />}
      <div className="actions">
        <Button variant="outline" onClick={() => setItems((current) => [...current, {
          key: crypto.randomUUID(), description: "", quantity: "1", unit_price_idr: "",
        }])}>Tambah item</Button>
        <Button type="submit" disabled={pending}>{pending ? "Menyimpan…" : "Simpan dan tinjau draft"}</Button>
        <Button variant="plain" onClick={onCancel} disabled={pending}>Batal</Button>
        {error?.status === 409 && onReload && <Button variant="outline" onClick={onReload}>Muat ulang penjualan</Button>}
      </div>
    </form>
  );
}
