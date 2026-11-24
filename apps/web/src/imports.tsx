import { useCallback, useState, type FormEvent } from "react";
import { api, failure, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Dialog } from "./components/ui/dialog";
import { Input } from "./components/ui/input";

export function ImportWorkspace({ csrf, owner }: { csrf: string; owner: boolean }) {
  const [selected, setSelected] = useState<string>();
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState<string>();
  const loadBatches = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/import-batches", { signal })), [revision]);
  const batches = useResource(loadBatches);
  return <section className="stack" aria-labelledby="imports-title">
    <div className="workspace-heading"><h2 id="imports-title">Import workbook historis</h2></div>
    <p className="muted">File XLSX masuk staging; belum mengubah ledger sampai baris ditinjau dan Owner menyetujui commit.</p>
    {notice && <p role="status" className="notice-success">{notice}</p>}
    <UploadForm csrf={csrf} onUploaded={(batch, duplicate) => {
      setSelected(batch.id); setRevision((value) => value + 1);
      setNotice(duplicate ? "File yang sama ditemukan; batch yang sudah ada dimuat kembali. Batch lama yang belum direview dapat diperbarui ke sumber sheet pelanggan." : "Berkas berhasil diproses ke staging untuk review.");
    }} />
    {owner && <section className="panel stack">
      <h3>Batch staging</h3>
      {batches.state.status === "loading" && <p role="status">Memuat batch…</p>}
      {batches.state.status === "error" && <ErrorNotice message={batches.state.error.message} retry={batches.reload} />}
      {batches.state.status === "ready" && (batches.state.data.data.length === 0 ? <p className="muted">Belum ada batch import.</p> : <ul className="customer-items">
        {batches.state.data.data.map((batch) => <li key={batch.id}><button className="customer-row" type="button" aria-current={selected === batch.id ? "true" : undefined} onClick={() => setSelected(batch.id)}>
          <span><strong>{batch.original_filename}</strong><span className="muted">{batch.status} · {batch.total_rows} baris · {batch.accepted_rows} diterima</span></span>
          <span className="muted">{batch.file_sha256.slice(0, 12)}…</span>
        </button></li>)}
      </ul>)}
    </section>}
    {owner && selected && <BatchReview key={selected} id={selected} csrf={csrf} refreshKey={revision} onChanged={() => setRevision((value) => value + 1)} />}
  </section>;
}

function UploadForm({ csrf, onUploaded }: { csrf: string; onUploaded: (batch: Schema["ImportBatchSummary"], duplicate: boolean) => void }) {
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);
  async function upload(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setError(undefined); setPending(true);
    const formElement = event.currentTarget;
    try {
      const file = new FormData(formElement).get("file");
      if (!(file instanceof File) || !file.name.toLowerCase().endsWith(".xlsx")) throw new Error("Pilih berkas .xlsx.");
      const form = new FormData(); form.append("file", file);
      const result = await api.POST("/api/v1/import-batches", {
        params: { header: { "X-CSRF-Token": csrf } },
        body: form as unknown as Schema["UploadImport"],
      });
      onUploaded(readResult(result).data, result.response.status === 200);
      formElement.reset();
    } catch (failureValue) { setError(failureValue instanceof Error ? failureValue.message : failure(failureValue).message); }
    finally { setPending(false); }
  }
  return <section className="panel stack"><h3>Unggah XLSX · maksimum 25 MiB</h3>
    <form className="date-form" onSubmit={upload}>
      <label>Berkas sumber<Input name="file" type="file" accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" required /></label>
      <Button type="submit" disabled={pending}>{pending ? "Memeriksa dan staging…" : "Unggah untuk review"}</Button>
    </form>
    {error && <ErrorNotice message={error} />}
  </section>;
}

function BatchReview({ id, csrf, refreshKey, onChanged }: { id: string; csrf: string; refreshKey: number; onChanged: () => void }) {
  const [refresh, setRefresh] = useState(0);
  const [selectedSheets, setSelectedSheets] = useState<string[]>([]);
  const load = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/import-batches/{id}", { params: { path: { id } }, signal })), [id, refresh, refreshKey]);
  const batch = useResource(load);
  const loadCustomerIdentities = useCallback(async (signal: AbortSignal) => {
    const prepared = readResult(await api.POST("/api/v1/import-batches/{id}/customer-identities/prepare", { params: { path: { id }, header: { "X-CSRF-Token": csrf } }, signal }));
    if (prepared.data.updated_sheets > 0) setRefresh((value) => value + 1);
    return readResult(await api.GET("/api/v1/import-batches/{id}/customer-identities", { params: { path: { id }, signal } }));
  }, [id, refresh, refreshKey, csrf]);
  const customerIdentities = useResource(loadCustomerIdentities);
  const [commitError, setCommitError] = useState<string>();
  const [committing, setCommitting] = useState(false);
  const [variance, setVariance] = useState("");
  const [varianceReason, setVarianceReason] = useState("");
  const identities = customerIdentities.state.status === "ready"
    ? [...customerIdentities.state.data.data].sort((a, b) => b.valid_rows - a.valid_rows || a.source_sheet.localeCompare(b.source_sheet, "id"))
    : [];
  const plannedCustomerCount = identities.filter((identity) => identity.resolution === "will_create").length;
  const unresolvedIdentityCount = customerIdentities.state.status === "ready" ? customerIdentities.state.data.data.filter((identity) => ["ambiguous", "missing_identity", "formula_identity", "inactive_match"].includes(identity.resolution)).length : 0;
  const selectedCustomers = identities.filter((identity) => selectedSheets.includes(identity.source_sheet));
  const selectedValidRows = selectedCustomers.reduce((sum, identity) => sum + identity.valid_rows, 0);
  const selectedUncomputableRows = selectedCustomers.reduce((sum, identity) => sum + identity.uncomputable_amount_rows, 0);
  const selectedPendingDebit = selectedCustomers.reduce((sum, identity) => sum + BigInt(identity.pending_debit_idr), 0n);
  const selectedPendingCredit = selectedCustomers.reduce((sum, identity) => sum + BigInt(identity.pending_credit_idr), 0n);
  const selectedValidDebit = selectedCustomers.reduce((sum, identity) => sum + BigInt(identity.valid_debit_idr), 0n);
  const selectedValidCredit = selectedCustomers.reduce((sum, identity) => sum + BigInt(identity.valid_credit_idr), 0n);
  const selectedKnownVariance = selectedCustomers.reduce((sum, identity) => sum + BigInt(identity.pending_debit_idr) - BigInt(identity.pending_credit_idr) - BigInt(identity.valid_debit_idr) + BigInt(identity.valid_credit_idr), 0n);
  const varianceRequired = selectedCustomers.length > 0 && (selectedUncomputableRows > 0 || selectedKnownVariance !== 0n);
  function canCommitIdentity(identity: Schema["ImportCustomerIdentityPreview"]): boolean {
    return identity.valid_rows > 0 && (["matched_code", "matched_name", "will_create"].includes(identity.resolution) || identity.mapped_valid_rows === identity.valid_rows);
  }
  function toggleCustomer(sheet: string, checked: boolean) {
    const nextSheets = checked ? [...new Set([...selectedSheets, sheet])] : selectedSheets.filter((item) => item !== sheet);
    const nextCustomers = identities.filter((identity) => nextSheets.includes(identity.source_sheet));
    const nextUnknownRows = nextCustomers.reduce((sum, identity) => sum + identity.uncomputable_amount_rows, 0);
    const nextVariance = nextCustomers.reduce((sum, identity) => sum + BigInt(identity.pending_debit_idr) - BigInt(identity.pending_credit_idr) - BigInt(identity.valid_debit_idr) + BigInt(identity.valid_credit_idr), 0n);
    setSelectedSheets(nextSheets);
    setVariance(nextUnknownRows === 0 && nextCustomers.length > 0 ? nextVariance.toString() : "");
    setVarianceReason(""); setCommitError(undefined);
  }
  async function commit() {
    if (batch.state.status !== "ready" || customerIdentities.state.status !== "ready" || !selectedCustomers.length) return;
    setCommitError(undefined); setCommitting(true);
    try {
      let allIds: string[] = [];
      for (const customer of selectedCustomers) {
        let next: string | null | undefined;
        do {
          const page = readResult(await api.GET("/api/v1/import-batches/{id}/rows", { params: { path: { id }, query: { ...(next ? { cursor: next } : {}), limit: 100, sheet_name: customer.source_sheet } } }));
          allIds = allIds.concat(page.data.filter((row) => row.review_status === "accepted" || row.review_status === "candidate").map((row) => row.id));
          next = page.meta.next_cursor;
        } while (next);
      }
      if (!allIds.length) throw new Error("Pilih pelanggan yang memiliki baris valid untuk diimpor.");
      if (varianceRequired && !variance.trim()) throw new Error("Masukkan variance hasil rekonsiliasi untuk pelanggan yang dipilih.");
      if (variance.trim() && variance.trim() !== "0" && !varianceReason.trim()) throw new Error("Masukkan alasan untuk variance nonnol.");
      const result = await api.POST("/api/v1/import-batches/{id}/commit", {
        params: { path: { id }, header: { "X-CSRF-Token": csrf, "Idempotency-Key": crypto.randomUUID() } },
        body: { expected_version: batch.state.data.data.version, row_ids: allIds, variance_idr: variance || undefined, variance_reason: varianceReason || undefined },
      });
      const data = readResult(result);
      const remaining = Math.max(0, batch.state.data.data.candidate_rows + batch.state.data.data.accepted_rows - selectedValidRows);
      setSelectedSheets([]); setVariance(""); setVarianceReason("");
      onChanged(); setRefresh((value) => value + 1);
      setCommitError(data.data.status === "reviewing" ? `Commit berhasil: ${data.data.committed_rows} baris dari ${selectedCustomers.length} pelanggan. Batch tetap terbuka; ${remaining.toLocaleString("id-ID")} baris valid lain atau exception masih dapat ditinjau nanti.` : `Import selesai: ${data.data.committed_rows} baris, ${data.data.created_customers} pelanggan baru, variance ${data.data.variance_idr}.`);
    } catch (value) { setCommitError(failure(value).message); }
    finally { setCommitting(false); }
  }
  return <section className="panel stack">
    <h3>Review batch</h3>
    {batch.state.status === "loading" && <p role="status">Memuat ringkasan batch…</p>}
    {batch.state.status === "error" && <ErrorNotice message={batch.state.error.message} retry={batch.reload} />}
    {batch.state.status === "ready" && <>
      <section className="import-summary" aria-label="Ringkasan keseluruhan batch">
        <div className="import-summary-heading"><div><span className="eyebrow">RINGKASAN KESELURUHAN</span><h4>{batch.state.data.data.original_filename}</h4></div><span className={`status-pill status-${batch.state.data.data.status}`}>{batch.state.data.data.status}</span></div>
        <div className="import-summary-grid">
          <SummaryMetric label="Total baris" value={batch.state.data.data.total_rows.toLocaleString("id-ID")} />
          <SummaryMetric label="Total saldo debit" value={formatRupiah(batch.state.data.data.source_debit_idr)} />
          <SummaryMetric label="Total saldo kredit" value={formatRupiah(batch.state.data.data.source_credit_idr)} />
          <SummaryMetric label="Nominal belum dapat dihitung" value={batch.state.data.data.uncomputable_amount_rows.toLocaleString("id-ID")} />
          <SummaryMetric label="Kandidat" value={batch.state.data.data.candidate_rows.toLocaleString("id-ID")} />
          <SummaryMetric label="Diterima" value={batch.state.data.data.accepted_rows.toLocaleString("id-ID")} />
          <SummaryMetric label="Karantina" value={batch.state.data.data.quarantined_rows.toLocaleString("id-ID")} />
          <SummaryMetric label="Dikecualikan" value={batch.state.data.data.excluded_rows.toLocaleString("id-ID")} />
          <SummaryMetric label="Sudah diimpor" value={batch.state.data.data.committed_rows.toLocaleString("id-ID")} />
        </div>
      </section>
      <p className="muted">Sumber import adalah sheet pelanggan bernama; sheet Analisa dan saldo formula tidak dipakai. Pilih pelanggan yang ingin diimpor sekarang. Pelanggan lain tetap di staging dan dapat dikomit nanti. {plannedCustomerCount} pelanggan baru akan dibuat otomatis saat pelanggan tersebut dikomit.</p>
      {customerIdentities.state.status === "loading" && <p role="status">Mencocokkan identitas pelanggan…</p>}
      {customerIdentities.state.status === "error" && <ErrorNotice message={customerIdentities.state.error.message} retry={customerIdentities.reload} />}
      {customerIdentities.state.status === "ready" && <section className="stack">
        <div><h4>Rekonsiliasi manual per pelanggan</h4><p className="muted">Kolom valid menunjukkan jumlah baris yang bisa langsung diimpor. Buka baris pelanggan untuk melihat transaksi dan exception-nya.</p></div>
        {customerIdentities.state.data.data.some((identity) => identity.source_sheet.trim().toLowerCase() === "analisa") && <p className="notice-error">Batch ini masih memakai aturan lama dan mencakup sheet Analisa. Unggah ulang file yang sama untuk memperbarui staging; batch hanya akan direset bila belum direview atau dikomit.</p>}
        {customerIdentities.state.data.data.every((identity) => identity.resolution === "missing_identity") && <p className="muted">Workbook tidak menyediakan identitas yang dapat dicocokkan untuk sheet ini. Baris memerlukan mapping Owner atau tetap dikarantina.</p>}
        <div className="table-scroll"><table className="import-table import-customer-summary"><thead><tr><th>Pilih</th><th>Sheet pelanggan</th><th>Kode</th><th>Nama pelanggan</th><th>Total baris</th><th>Total debit sumber</th><th>Total kredit sumber</th><th>Net saldo sumber</th><th>Valid untuk diimpor</th><th>Karantina / dikecualikan</th><th>Nominal belum dihitung</th><th>Net baris valid</th><th>Variance pending</th><th>Pencocokan</th><th>Pelanggan tujuan</th><th>Data pelanggan</th></tr></thead><tbody>
          {customerIdentities.state.data.data.map((identity) => <CustomerIdentityRow key={identity.source_sheet} batchId={id} identity={identity} csrf={csrf} selected={selectedSheets.includes(identity.source_sheet)} selectable={canCommitIdentity(identity)} onSelected={(checked) => toggleCustomer(identity.source_sheet, checked)} onChanged={() => { setRefresh((value) => value + 1); onChanged(); }} />)}
        </tbody></table></div>
      </section>}
      {selectedCustomers.length > 0 && <section className="panel stack" aria-label="Commit pelanggan terpilih">
        <h4>Commit pelanggan terpilih</h4>
        <p>{selectedCustomers.length} pelanggan dipilih · {selectedValidRows.toLocaleString("id-ID")} baris valid akan diimpor. Baris valid dari pelanggan lain tetap tersimpan untuk nanti.</p>
        <dl className="aging-summary variance-explanation">
          <div><dt>Net sumber yang belum diimpor</dt><dd>{formatRupiah((selectedPendingDebit - selectedPendingCredit).toString())}</dd></div>
          <div><dt>Net baris valid pilihan ini</dt><dd>{formatRupiah((selectedValidDebit - selectedValidCredit).toString())}</dd></div>
          <div><dt>Variance terhitung</dt><dd>{selectedUncomputableRows > 0 ? "Belum dapat dihitung" : formatRupiah(selectedKnownVariance.toString())}</dd></div>
        </dl>
        <p className="muted">Rumus variance: (debit sumber − kredit sumber) − (debit baris valid − kredit baris valid). Jadi variance bukan saldo/net pelanggan. Jika semua nominal terhitung, sistem mengisi selisihnya otomatis; Owner cukup memastikan angkanya. Variance nonnol tetap memerlukan alasan.</p>
        {selectedUncomputableRows > 0 && <p className="notice-error">Pelanggan terpilih memiliki {selectedUncomputableRows} baris dengan nominal belum dapat dihitung. Periksa exception di dalam data pelanggan; untuk commit, masukkan variance hasil rekonsiliasi Owner. Jangan menebak.</p>}
        {varianceRequired && <div className="date-form"><label>Variance IDR (wajib untuk pilihan ini)<Input inputMode="numeric" value={variance} onChange={(event) => setVariance(event.target.value)} placeholder={selectedUncomputableRows > 0 ? "Isi setelah rekonsiliasi Owner" : selectedKnownVariance.toString()} /></label><label>Alasan variance {variance !== "" && variance !== "0" ? "(wajib)" : "(bila ada selisih)"}<Input value={varianceReason} onChange={(event) => setVarianceReason(event.target.value)} maxLength={1000} /></label></div>}
        <Button onClick={commit} disabled={committing || selectedCustomers.some((identity) => !canCommitIdentity(identity))}>{committing ? "Menyimpan pelanggan terpilih…" : `Commit ${selectedValidRows.toLocaleString("id-ID")} baris valid`}</Button>
      </section>}
      {batch.state.data.data.status === "reviewing" && batch.state.data.data.candidate_rows + batch.state.data.data.accepted_rows === 0 && <p className="notice-success">Tidak ada baris valid yang menunggu commit. Semua baris yang dapat dihitung sudah diproses.</p>}
      {customerIdentities.state.status === "ready" && unresolvedIdentityCount > 0 && <p className="muted">{unresolvedIdentityCount} identitas sheet belum dapat dipastikan. Baris terkait tetap dikarantina kecuali Owner memetakannya manual.</p>}
    </>}
    {commitError && <p role="status">{commitError}</p>}
  </section>;
}

function CustomerIdentityRow({ identity, batchId, csrf, selected, selectable, onSelected, onChanged }: {
  identity: Schema["ImportCustomerIdentityPreview"];
  batchId: string;
  csrf: string;
  selected: boolean;
  selectable: boolean;
  onSelected: (checked: boolean) => void;
  onChanged: () => void;
}) {
  const [open, setOpen] = useState(false);
  const exceptionRows = identity.quarantined_rows + identity.excluded_rows;
  const netValid = BigInt(identity.valid_debit_idr) - BigInt(identity.valid_credit_idr);
  const netSource = BigInt(identity.source_debit_idr) - BigInt(identity.source_credit_idr);
  const pendingVariance = BigInt(identity.pending_debit_idr) - BigInt(identity.pending_credit_idr) - BigInt(identity.valid_debit_idr) + BigInt(identity.valid_credit_idr);
  return <>
    <tr>
      <td><input type="checkbox" aria-label={`Pilih ${identity.source_name || identity.source_sheet} untuk commit`} checked={selected} disabled={!selectable} onChange={(event) => onSelected(event.target.checked)} /></td>
      <td>{identity.source_sheet}</td>
      <td>{identity.source_code || "—"}</td>
      <td><strong>{identity.source_name || "Nama belum terdeteksi"}</strong></td>
      <td>{identity.source_rows.toLocaleString("id-ID")}</td>
      <td>{formatRupiah(identity.source_debit_idr)}</td>
      <td>{formatRupiah(identity.source_credit_idr)}</td>
      <td><strong>{formatRupiah(netSource.toString())}</strong></td>
      <td><strong>{identity.valid_rows.toLocaleString("id-ID")}</strong><span className="table-subline">{identity.candidate_rows} kandidat · {identity.accepted_rows} diterima</span></td>
      <td>{exceptionRows.toLocaleString("id-ID")}<span className="table-subline">{identity.quarantined_rows} karantina · {identity.excluded_rows} dikecualikan</span></td>
      <td>{identity.uncomputable_amount_rows.toLocaleString("id-ID")}</td>
      <td><strong>{formatRupiah(netValid.toString())}</strong><span className="table-subline">debit valid − kredit valid</span></td>
      <td>{identity.uncomputable_amount_rows > 0 ? <span className="badge">Perlu rekonsiliasi</span> : <strong>{formatRupiah(pendingVariance.toString())}</strong>}<span className="table-subline">sumber belum diimpor − baris valid</span></td>
      <td>{customerResolutionLabel(identity.resolution)}{selectable && identity.mapped_valid_rows === identity.valid_rows && !["matched_code", "matched_name", "will_create"].includes(identity.resolution) && <span className="table-subline">Semua baris valid sudah dipetakan</span>}{!selectable && identity.valid_rows > 0 && <span className="table-subline">Mapping seluruh baris valid diperlukan</span>}</td>
      <td>{identity.customer_name || (identity.resolution === "will_create" ? "Dibuat saat commit" : "—")}</td>
      <td><Button type="button" variant="outline" onClick={() => setOpen(true)}>{identity.valid_rows > 0 ? `Buka ${identity.valid_rows} baris valid` : "Buka review"}</Button></td>
    </tr>
    <Dialog open={open} onOpenChange={setOpen} title={`Data pelanggan · ${identity.source_name || identity.source_sheet}`}>
      <CustomerRowsDialog identity={identity} batchId={batchId} csrf={csrf} onChanged={onChanged} />
    </Dialog>
  </>;
}

function CustomerRowsDialog({ identity, batchId, csrf, onChanged }: {
  identity: Schema["ImportCustomerIdentityPreview"];
  batchId: string;
  csrf: string;
  onChanged: () => void;
}) {
  const [cursorStack, setCursorStack] = useState<string[]>([]);
  const [reviewStatus, setReviewStatus] = useState(identity.valid_rows > 0 ? "valid" : "quarantined");
  const [refresh, setRefresh] = useState(0);
  const cursor = cursorStack.at(-1);
  const loadRows = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/import-batches/{id}/rows", {
    params: { path: { id: batchId }, query: { ...(cursor ? { cursor } : {}), limit: 50, sheet_name: identity.source_sheet, review_status: reviewStatus || undefined }, signal },
  })), [batchId, cursor, identity.source_sheet, refresh, reviewStatus]);
  const rows = useResource(loadRows);
  return <section className="stack customer-row-dialog-list">
    <div className="import-summary-grid">
      <SummaryMetric label="Baris valid" value={identity.valid_rows.toLocaleString("id-ID")} />
      <SummaryMetric label="Karantina" value={identity.quarantined_rows.toLocaleString("id-ID")} />
      <SummaryMetric label="Dikecualikan" value={identity.excluded_rows.toLocaleString("id-ID")} />
      <SummaryMetric label="Sudah diimpor" value={identity.committed_rows.toLocaleString("id-ID")} />
    </div>
    <p className="muted">Baris kandidat dan diterima dapat dipilih untuk commit. Untuk memperbaiki baris bermasalah, tekan <strong>Tinjau</strong>; pilih <strong>Override Owner</strong> hanya jika nilai transaksi dapat diverifikasi dari sumber.</p>
    <label>Filter baris<select className="input" value={reviewStatus} onChange={(event) => { setReviewStatus(event.target.value); setCursorStack([]); }}>
      <option value="">Semua baris pelanggan</option><option value="valid">Semua baris valid</option><option value="candidate">Kandidat</option><option value="accepted">Diterima</option><option value="quarantined">Karantina</option><option value="excluded">Dikecualikan</option><option value="committed">Sudah diimpor</option>
    </select></label>
    {rows.state.status === "loading" && <p role="status">Memuat baris pelanggan…</p>}
    {rows.state.status === "error" && <ErrorNotice message={rows.state.error.message} retry={rows.reload} />}
    {rows.state.status === "ready" && (rows.state.data.data.length === 0 ? <p className="muted">Tidak ada baris pada filter ini.</p> : <div className="table-scroll import-table-wrap"><table className="import-table"><thead><tr><th>Baris sumber</th><th>Pelanggan</th><th>Debit</th><th>Kredit</th><th>Status</th><th>Perlu diperiksa</th><th>Aksi</th></tr></thead><tbody>
      {rows.state.data.data.map((row) => <ImportRowCard key={row.id} batchId={batchId} row={row} csrf={csrf} onSaved={() => { setRefresh((value) => value + 1); onChanged(); }} />)}
    </tbody></table></div>)}
    {rows.state.status === "ready" && (cursorStack.length > 0 || rows.state.data.meta.next_cursor) && <div className="pagination">
      <Button type="button" variant="outline" disabled={!cursorStack.length} onClick={() => setCursorStack((stack) => stack.slice(0, -1))}>Sebelumnya</Button>
      <Button type="button" variant="outline" disabled={!rows.state.data.meta.next_cursor} onClick={() => { const next = rows.state.status === "ready" ? rows.state.data.meta.next_cursor : undefined; if (next) setCursorStack((stack) => [...stack, next]); }}>Berikutnya</Button>
    </div>}
  </section>;
}

function ImportRowCard({ batchId, row, csrf, onSaved }: { batchId: string; row: Schema["ImportRow"]; csrf: string; onSaved: () => void }) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [action, setAction] = useState<Schema["ImportReviewAction"]>(row.review_status === "quarantined" ? "override" : "accept");
  const [customerId, setCustomerId] = useState(row.mapped_customer_id ?? "");
  const [customerSearch, setCustomerSearch] = useState("");
  const [reason, setReason] = useState("");
  const [note, setNote] = useState(row.review_note ?? "");
  const [overrideValues, setOverrideValues] = useState(() => Object.fromEntries(OVERRIDE_FIELDS.map(({ key }) => [key, candidateField(row.parsed_values, key)])) as Record<OverrideKey, string>);
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);
  const loadCustomers = useCallback(async (signal: AbortSignal) => readResult(await api.GET("/api/v1/customers", { params: { query: { search: customerSearch, limit: 25 } }, signal })), [customerSearch]);
  const customers = useResource(loadCustomers);
  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setError(undefined); setPending(true);
    try {
      const overrides = action === "override" ? Object.fromEntries(OVERRIDE_FIELDS.filter(({ key }) => overrideValues[key].trim() !== "").map(({ key }) => [key, overrideValues[key].trim()])) : undefined;
      const result = await api.PATCH("/api/v1/import-batches/{id}/rows/{row_id}", {
        params: { path: { id: batchId, row_id: row.id }, header: { "X-CSRF-Token": csrf } },
        body: { expected_version: row.version, action, mapped_customer_id: customerId || undefined, overrides, review_note: note || undefined, reason },
      });
      readResult(result); onSaved();
    } catch (value) { setError(value instanceof SyntaxError ? "JSON override tidak valid." : failure(value).message); }
    finally { setPending(false); }
  }
  const parsed = asRecord(row.parsed_values);
  const issueText = Array.isArray(row.issues) && row.issues.length ? row.issues.map(issueLabel).join(" · ") : "Tidak ada issue otomatis";
  return <>
    <tr>
      <td>{row.sheet_name} · {row.row_number}<span className="table-subline">{String(parsed.effective_date ?? "Tanggal tidak tersedia")}</span></td>
      <td>{row.source_customer.name || "Nama belum terdeteksi"}{row.source_customer.code ? <span className="table-subline">Kode {row.source_customer.code}</span> : null}</td>
      <td>{displayMoney(parsed.debit_idr)}</td><td>{displayMoney(parsed.credit_idr)}</td>
      <td><span className="badge">{reviewStatusLabel(row.review_status)}</span></td><td className="issue-cell">{issueText}</td>
      <td><Button variant="outline" onClick={() => setDialogOpen(true)}>Tinjau</Button></td>
    </tr>
    <Dialog open={dialogOpen} onOpenChange={setDialogOpen} title={`Review ${row.sheet_name} · baris ${row.row_number}`}>
    <article className="stack">
    <div className="workspace-heading"><strong>{row.sheet_name} · baris {row.row_number}</strong><span className="badge">{reviewStatusLabel(row.review_status)}</span></div>
    <p><strong>Perlu diperiksa:</strong> {Array.isArray(row.issues) && row.issues.length ? row.issues.map(issueLabel).join(" · ") : "Tidak ada issue otomatis"}</p>
    <p className="muted">Pelanggan sumber: {row.source_customer.name || "Nama belum terdeteksi"}{row.source_customer.code ? ` · kode ${row.source_customer.code}` : ""}</p>
    <details><summary>Lihat nilai sumber dan kandidat</summary>
      <div className="table-scroll"><table><thead><tr><th>Kolom</th><th>Nilai sumber</th><th>Formula</th></tr></thead><tbody>
        {sourceCells(row.raw_values).map((cell, index) => <tr key={`${cell.column}-${index}`}><td>{cell.column}</td><td>{displayValue(cell.value)}</td><td>{cell.formula ? "Ya · nilai sumber diabaikan" : "Tidak"}</td></tr>)}
      </tbody></table></div>
      <dl className="aging-summary">{Object.entries(asRecord(row.parsed_values)).map(([key, value]) => <div key={key}><dt>{candidateLabel(key)}</dt><dd>{displayValue(value)}</dd></div>)}</dl>
    </details>
    {row.review_status !== "committed" && <form className="stack" onSubmit={save}>
      <div className="date-form">
        <label>Aksi review<select className="input" value={action} onChange={(event) => setAction(event.target.value as Schema["ImportReviewAction"])}>
          <option value="accept">Terima</option><option value="override">Override Owner</option><option value="quarantine">Karantina</option><option value="exclude">Kecualikan</option>
        </select></label>
        <label>Cari pelanggan<Input value={customerSearch} onChange={(event) => setCustomerSearch(event.target.value)} placeholder="Nama atau kode" /></label>
        <label>Pelanggan tujuan<select className="input" value={customerId} onChange={(event) => setCustomerId(event.target.value)}>
          <option value="">Pilih pelanggan</option>{customers.state.status === "ready" && customers.state.data.data.items.map((item) => { const customer = "customer" in item ? item.customer : item; return <option key={customer.id} value={customer.id}>{customer.name}{customer.code ? ` · ${customer.code}` : ""}</option>; })}
        </select></label>
      </div>
      {action === "override" && <fieldset className="stack"><legend>Perbaiki kandidat transaksi</legend><p className="muted">Nominal dimasukkan sebagai rupiah bulat. Formula Excel tidak disalin; isi nilai yang sudah diverifikasi dari sumber.</p><div className="date-form">
        {OVERRIDE_FIELDS.map((field) => <label key={field.key}>{field.label}<Input type={field.type} inputMode={field.inputMode} pattern={field.pattern} required={field.required} value={overrideValues[field.key]} onChange={(event) => setOverrideValues((current) => ({ ...current, [field.key]: event.target.value }))} /></label>)}
      </div></fieldset>}
      <label>Catatan review / usulan split (tetap satu event per baris)<textarea className="input" rows={2} maxLength={1000} value={note} onChange={(event) => setNote(event.target.value)} /></label>
      <label>Alasan review<Input required maxLength={1000} value={reason} onChange={(event) => setReason(event.target.value)} /></label>
      {error && <ErrorNotice message={error} />}
      <Button type="submit" disabled={pending}>{pending ? "Menyimpan review…" : "Simpan review"}</Button>
    </form>}
    </article>
    </Dialog>
  </>;
}

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return <div className="import-metric"><span>{label}</span><strong>{value}</strong></div>;
}

function formatRupiah(value: string): string {
  if (!/^-?\d+$/.test(value)) return `Rp ${value}`;
  return new Intl.NumberFormat("id-ID", { style: "currency", currency: "IDR", maximumFractionDigits: 0 }).format(BigInt(value));
}

function subtractMoney(debit: string, credit: string): string {
  if (!/^\d+$/.test(debit) || !/^\d+$/.test(credit)) return "0";
  return (BigInt(debit) - BigInt(credit)).toString();
}

function displayMoney(value: unknown): string {
  if (value === null || value === undefined || value === "") return "—";
  return formatRupiah(String(value));
}

type OverrideKey = "effective_date" | "reference_no" | "description" | "quantity" | "unit_price_idr" | "debit_idr" | "credit_idr";
const OVERRIDE_FIELDS: { key: OverrideKey; label: string; type?: string; inputMode?: "text" | "numeric" | "decimal"; pattern?: string; required?: boolean }[] = [
  { key: "effective_date", label: "Tanggal transaksi", type: "date", required: true },
  { key: "reference_no", label: "Nomor faktur" },
  { key: "description", label: "Uraian" },
  { key: "quantity", label: "Jumlah", inputMode: "decimal", pattern: "[0-9]+([.][0-9]{1,3})?" },
  { key: "unit_price_idr", label: "Harga satuan (rupiah)", inputMode: "numeric", pattern: "[0-9]+" },
  { key: "debit_idr", label: "Debit (rupiah)", inputMode: "numeric", pattern: "[0-9]+", required: true },
  { key: "credit_idr", label: "Kredit (rupiah)", inputMode: "numeric", pattern: "[0-9]+", required: true },
];

const ISSUE_LABELS: Record<string, string> = {
  transaction_data_issue: "Perlu tinjauan transaksi",
  invalid_or_missing_date: "Tanggal kosong atau tidak valid",
  invalid_amount: "Nominal kosong atau tidak valid",
  invalid_debit_credit: "Debit/kredit harus memiliki satu nominal positif",
  future_business_date: "Tanggal berada di masa depan",
  inherited_previous_date: "Tanggal mengikuti tanggal sebelumnya pada sheet ini",
  mixed_debit_credit: "Debit dan kredit terisi bersamaan",
  quantity_price_mismatch: "Jumlah × harga berbeda dari debit",
  cell_error: "Sel sumber berisi error Excel",
  formula_customer_identity: "Identitas pelanggan berasal dari formula",
  customer_identity_unresolved: "Identitas pelanggan belum terselesaikan",
  formula_value_ignored: "Nilai formula diabaikan",
  duplicate_source_row: "Baris sumber duplikat",
  non_transaction_row: "Bukan baris transaksi",
};

function issueLabel(issue: unknown): string {
  const code = String(issue);
  return ISSUE_LABELS[code] ?? code;
}

function reviewStatusLabel(status: string): string {
  return ({ candidate: "Kandidat transaksi", accepted: "Diterima", quarantined: "Karantina", excluded: "Dikecualikan", committed: "Sudah diimpor" } as Record<string, string>)[status] ?? status;
}

function customerResolutionLabel(resolution: string): string {
  return ({ matched_code: "Cocok berdasarkan kode", matched_name: "Cocok berdasarkan nama", will_create: "Pelanggan baru saat commit", ambiguous: "Ambigu · perlu ditinjau", missing_identity: "Identitas tidak lengkap", formula_identity: "Identitas berisi formula", inactive_match: "Pelanggan nonaktif" } as Record<string, string>)[resolution] ?? resolution;
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function candidateField(candidate: unknown, field: OverrideKey): string {
  const value = asRecord(candidate)[field];
  return value === null || value === undefined ? "" : String(value);
}

function candidateLabel(key: string): string {
  return ({ effective_date: "Tanggal", reference_no: "Nomor faktur", description: "Uraian", quantity: "Jumlah", unit_price_idr: "Harga satuan", debit_idr: "Debit", credit_idr: "Kredit" } as Record<string, string>)[key] ?? key;
}

function displayValue(value: unknown): string {
  if (typeof value === "string") return value;
  if (value === null || value === undefined) return "—";
  return JSON.stringify(value) ?? String(value);
}

function sourceCells(value: unknown): { column: string; value: unknown; formula: boolean }[] {
  if (!Array.isArray(value)) return [];
  return value.map(asRecord).filter((cell) => typeof cell.column === "string").map((cell) => ({ column: String(cell.column), value: cell.value, formula: cell.formula === true }));
}
