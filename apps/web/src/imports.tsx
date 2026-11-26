import { useCallback, useState, type FormEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  Upload,
  FileSpreadsheet,
  AlertCircle,
  CheckCircle2,
  List,
  Layers,
  ChevronRight,
  ExternalLink,
} from "lucide-react";
import { api, failure, readResult, useResource, type Schema } from "./api";
import { ErrorNotice } from "./components/presentation";
import { Button } from "./components/ui/button";
import { Dialog } from "./components/ui/dialog";
import { Input } from "./components/ui/input";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { MoneyDisplay } from "./components/money-display";

export function ImportWorkspace({ csrf, owner }: { csrf: string; owner: boolean }) {
  const { batchId } = useParams();
  const navigate = useNavigate();
  const selected = batchId;
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState<string>();

  const loadBatches = useCallback(
    async (signal: AbortSignal) => readResult(await api.GET("/api/v1/import-batches", { signal })),
    [revision],
  );
  const batches = useResource(loadBatches);

  return (
    <div className="space-y-6">
      <Card className="border-slate-200/90 shadow-2xs">
        <CardHeader className="border-b border-slate-100 pb-4">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-2">
                <FileSpreadsheet className="h-4 w-4 text-emerald-800" />
                <CardTitle className="text-base font-bold text-slate-900">
                  Import Workbook Historis (Excel XLSX)
                </CardTitle>
              </div>
              <p className="text-xs text-slate-500 mt-0.5">
                Unggah catatan keuangan Excel untuk ditelaah di staging sebelum dimasukkan resmi ke buku piutang.
              </p>
            </div>

            {selected && owner && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => navigate("/imports")}
                className="gap-1.5 text-xs text-slate-700"
              >
                <List className="h-3.5 w-3.5" />
                <span>Lihat Semua Batch</span>
              </Button>
            )}
          </div>
        </CardHeader>

        <CardContent className="p-6 space-y-6">
          {notice && (
            <div className="flex items-center gap-2 p-3.5 rounded-xl border border-emerald-200 bg-emerald-50 text-emerald-900 text-xs font-medium">
              <CheckCircle2 className="h-4 w-4 text-emerald-600 shrink-0" />
              <span>{notice}</span>
            </div>
          )}

          <UploadForm
            csrf={csrf}
            onUploaded={(batch, duplicate) => {
              navigate(`/imports/${batch.id}`);
              setRevision((val) => val + 1);
              setNotice(
                duplicate
                  ? "File yang sama ditemukan; batch sebelumnya dimuat kembali untuk ditinjau."
                  : "Berkas Excel berhasil diunggah ke staging untuk proses telaah (review).",
              );
            }}
          />

          {owner && (
            <div className="space-y-3 pt-2">
              <div className="flex items-center justify-between">
                <h4 className="text-xs font-bold text-slate-700 uppercase tracking-wider flex items-center gap-1.5">
                  <Layers className="h-3.5 w-3.5 text-slate-400" />
                  <span>Daftar Batch Staging</span>
                </h4>
              </div>

              {batches.state.status === "loading" && (
                <div className="p-8 text-center text-xs text-slate-500">Memuat daftar batch…</div>
              )}

              {batches.state.status === "error" && (
                <ErrorNotice message={batches.state.error.message} retry={batches.reload} />
              )}

              {batches.state.status === "ready" && (
                batches.state.data.data.length === 0 ? (
                  <p className="text-xs text-slate-500 py-4 text-center">Belum ada batch import yang diunggah.</p>
                ) : (
                  <div className="divide-y divide-slate-100 rounded-xl border border-slate-200 bg-white shadow-2xs overflow-hidden">
                    {batches.state.data.data.map((batch) => {
                      const isCurrent = selected === batch.id;
                      return (
                        <button
                          key={batch.id}
                          type="button"
                          onClick={() => navigate(`/imports/${batch.id}`)}
                          className={`w-full text-left p-4 flex items-center justify-between gap-4 transition-colors cursor-pointer ${
                            isCurrent ? "bg-emerald-50/60 border-l-4 border-l-emerald-800" : "hover:bg-slate-50"
                          }`}
                        >
                          <div className="space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="font-semibold text-xs text-slate-900">{batch.original_filename}</span>
                              <Badge
                                variant={
                                  batch.status === "committed"
                                    ? "success"
                                    : batch.status === "reviewing"
                                    ? "warning"
                                    : "secondary"
                                }
                                className="text-[10px]"
                              >
                                {batch.status}
                              </Badge>
                            </div>
                            <div className="flex items-center gap-2 text-[11px] text-slate-500 font-mono">
                              <span>Total {batch.total_rows.toLocaleString("id-ID")} baris</span>
                              <span>•</span>
                              <span>{batch.accepted_rows.toLocaleString("id-ID")} diterima</span>
                              <span>•</span>
                              <span>SHA: {batch.file_sha256.slice(0, 8)}…</span>
                            </div>
                          </div>
                          <ChevronRight className="h-4 w-4 text-slate-400" />
                        </button>
                      );
                    })}
                  </div>
                )
              )}
            </div>
          )}

          {owner && selected && (
            <div className="pt-4 border-t border-slate-200">
              <BatchReview
                key={selected}
                id={selected}
                csrf={csrf}
                refreshKey={revision}
                onChanged={() => setRevision((value) => value + 1)}
              />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function UploadForm({
  csrf,
  onUploaded,
}: {
  csrf: string;
  onUploaded: (batch: Schema["ImportBatchSummary"], duplicate: boolean) => void;
}) {
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);

  async function upload(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    setPending(true);
    const formElement = event.currentTarget;
    try {
      const file = new FormData(formElement).get("file");
      if (!(file instanceof File) || !file.name.toLowerCase().endsWith(".xlsx")) {
        throw new Error("Pilih berkas format .xlsx yang valid.");
      }
      const form = new FormData();
      form.append("file", file);
      const result = await api.POST("/api/v1/import-batches", {
        params: { header: { "X-CSRF-Token": csrf } },
        body: form as unknown as Schema["UploadImport"],
      });
      onUploaded(readResult(result).data, result.response.status === 200);
      formElement.reset();
    } catch (failureValue) {
      setError(failureValue instanceof Error ? failureValue.message : failure(failureValue).message);
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="p-5 rounded-2xl border-2 border-dashed border-emerald-300/80 bg-emerald-50/20">
      <form onSubmit={upload} className="flex flex-col sm:flex-row items-center gap-4">
        <div className="flex-1 w-full space-y-1">
          <label className="text-xs font-bold text-slate-800 flex items-center gap-1.5" htmlFor="upload-xlsx">
            <Upload className="h-4 w-4 text-emerald-800" />
            <span>Pilih Berkas Excel Workbook Toko (.xlsx, maks. 25 MiB)</span>
          </label>
          <Input
            id="upload-xlsx"
            name="file"
            type="file"
            accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            required
            className="text-xs file:mr-3 file:py-1 file:px-2.5 file:rounded-md file:border-0 file:text-xs file:font-semibold file:bg-emerald-100 file:text-emerald-900 hover:file:bg-emerald-200"
          />
        </div>

        <Button type="submit" disabled={pending} className="gap-2 shrink-0 font-semibold self-end sm:self-auto">
          <Upload className="h-4 w-4" />
          <span>{pending ? "Memproses Staging…" : "Unggah & Telaah Staging"}</span>
        </Button>
      </form>

      {error && <div className="mt-3"><ErrorNotice message={error} /></div>}
    </div>
  );
}

function BatchReview({
  id,
  csrf,
  refreshKey,
  onChanged,
}: {
  id: string;
  csrf: string;
  refreshKey: number;
  onChanged: () => void;
}) {
  const [refresh, setRefresh] = useState(0);
  const [selectedSheets, setSelectedSheets] = useState<string[]>([]);

  const load = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/import-batches/{id}", {
          params: { path: { id } },
          signal,
        }),
      ),
    [id, refresh, refreshKey],
  );
  const batch = useResource(load);

  const loadCustomerIdentities = useCallback(
    async (signal: AbortSignal) => {
      const prepared = readResult(
        await api.POST("/api/v1/import-batches/{id}/customer-identities/prepare", {
          params: { path: { id }, header: { "X-CSRF-Token": csrf } },
          signal,
        }),
      );
      if (prepared.data.updated_sheets > 0) setRefresh((val) => val + 1);
      return readResult(
        await api.GET("/api/v1/import-batches/{id}/customer-identities", {
          params: { path: { id }, signal },
        }),
      );
    },
    [id, refresh, refreshKey, csrf],
  );
  const customerIdentities = useResource(loadCustomerIdentities);

  const [commitError, setCommitError] = useState<string>();
  const [committing, setCommitting] = useState(false);
  const [variance, setVariance] = useState("");
  const [varianceReason, setVarianceReason] = useState("");

  const identities =
    customerIdentities.state.status === "ready"
      ? [...customerIdentities.state.data.data].sort(
          (a, b) => b.valid_rows - a.valid_rows || a.source_sheet.localeCompare(b.source_sheet, "id"),
        )
      : [];

  const plannedCustomerCount = identities.filter((i) => i.resolution === "will_create").length;
  const selectedCustomers = identities.filter((i) => selectedSheets.includes(i.source_sheet));
  const selectedValidRows = selectedCustomers.reduce((sum, i) => sum + i.valid_rows, 0);
  const selectedUncomputableRows = selectedCustomers.reduce((sum, i) => sum + i.uncomputable_amount_rows, 0);
  const selectedPendingDebit = selectedCustomers.reduce((sum, i) => sum + BigInt(i.pending_debit_idr), 0n);
  const selectedPendingCredit = selectedCustomers.reduce((sum, i) => sum + BigInt(i.pending_credit_idr), 0n);
  const selectedValidDebit = selectedCustomers.reduce((sum, i) => sum + BigInt(i.valid_debit_idr), 0n);
  const selectedValidCredit = selectedCustomers.reduce((sum, i) => sum + BigInt(i.valid_credit_idr), 0n);
  const selectedKnownVariance =
    selectedPendingDebit - selectedPendingCredit - selectedValidDebit + selectedValidCredit;
  const varianceRequired = selectedCustomers.length > 0 && (selectedUncomputableRows > 0 || selectedKnownVariance !== 0n);

  function canCommitIdentity(identity: Schema["ImportCustomerIdentityPreview"]): boolean {
    return (
      identity.valid_rows > 0 &&
      (["matched_code", "matched_name", "will_create"].includes(identity.resolution) ||
        identity.mapped_valid_rows === identity.valid_rows)
    );
  }

  function toggleCustomer(sheet: string, checked: boolean) {
    const nextSheets = checked
      ? [...new Set([...selectedSheets, sheet])]
      : selectedSheets.filter((item) => item !== sheet);
    const nextCustomers = identities.filter((identity) => nextSheets.includes(identity.source_sheet));
    const nextUnknownRows = nextCustomers.reduce((sum, identity) => sum + identity.uncomputable_amount_rows, 0);
    const nextVariance = nextCustomers.reduce(
      (sum, identity) =>
        sum +
        BigInt(identity.pending_debit_idr) -
        BigInt(identity.pending_credit_idr) -
        BigInt(identity.valid_debit_idr) +
        BigInt(identity.valid_credit_idr),
      0n,
    );
    setSelectedSheets(nextSheets);
    setVariance(nextUnknownRows === 0 && nextCustomers.length > 0 ? nextVariance.toString() : "");
    setVarianceReason("");
    setCommitError(undefined);
  }

  async function commit() {
    if (batch.state.status !== "ready" || customerIdentities.state.status !== "ready" || !selectedCustomers.length)
      return;
    setCommitError(undefined);
    setCommitting(true);
    try {
      let allIds: string[] = [];
      for (const customer of selectedCustomers) {
        let next: string | null | undefined;
        do {
          const page = readResult(
            await api.GET("/api/v1/import-batches/{id}/rows", {
              params: {
                path: { id },
                query: { ...(next ? { cursor: next } : {}), limit: 100, sheet_name: customer.source_sheet },
              },
            }),
          );
          allIds = allIds.concat(
            page.data
              .filter((row) => row.review_status === "accepted" || row.review_status === "candidate")
              .map((row) => row.id),
          );
          next = page.meta.next_cursor;
        } while (next);
      }

      if (!allIds.length) throw new Error("Pilih pelanggan yang memiliki baris transaksi valid.");
      if (varianceRequired && !variance.trim())
        throw new Error("Masukkan nilai variance hasil rekonsiliasi untuk pelanggan terpilih.");
      if (variance.trim() && variance.trim() !== "0" && !varianceReason.trim())
        throw new Error("Masukkan alasan catatan audit untuk variance non-nol.");

      const result = await api.POST("/api/v1/import-batches/{id}/commit", {
        params: {
          path: { id },
          header: { "X-CSRF-Token": csrf, "Idempotency-Key": crypto.randomUUID() },
        },
        body: {
          expected_version: batch.state.data.data.version,
          row_ids: allIds,
          variance_idr: variance || undefined,
          variance_reason: varianceReason || undefined,
        },
      });

      const data = readResult(result);
      setSelectedSheets([]);
      setVariance("");
      setVarianceReason("");
      onChanged();
      setRefresh((val) => val + 1);
      setCommitError(
        data.data.status === "reviewing"
          ? `Commit berhasil: ${data.data.committed_rows} baris dari ${selectedCustomers.length} pelanggan telah masuk ledger. Batch tetap dibuka untuk baris sisa.`
          : `Import tuntas: ${data.data.committed_rows} baris berhasil dimasukkan ke buku piutang.`,
      );
    } catch (err) {
      setCommitError(failure(err).message);
    } finally {
      setCommitting(false);
    }
  }

  return (
    <div className="space-y-6">
      {batch.state.status === "loading" && <p className="text-xs text-slate-500">Memuat rincian batch…</p>}
      {batch.state.status === "error" && <ErrorNotice message={batch.state.error.message} retry={batch.reload} />}

      {batch.state.status === "ready" && (
        <div className="space-y-6">
          {/* Batch Metrics Grid */}
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
            <div className="p-3.5 rounded-xl border border-slate-200 bg-white">
              <span className="text-[11px] text-slate-500 font-medium">Total Baris</span>
              <p className="font-mono text-base font-bold text-slate-900 mt-0.5">
                {batch.state.data.data.total_rows.toLocaleString("id-ID")}
              </p>
            </div>
            <div className="p-3.5 rounded-xl border border-slate-200 bg-white">
              <span className="text-[11px] text-slate-500 font-medium">Debit Sumber</span>
              <p className="font-mono text-sm font-bold text-amber-900 mt-0.5">
                <MoneyDisplay amount={batch.state.data.data.source_debit_idr} size="sm" variant="debit" />
              </p>
            </div>
            <div className="p-3.5 rounded-xl border border-slate-200 bg-white">
              <span className="text-[11px] text-slate-500 font-medium">Kredit Sumber</span>
              <p className="font-mono text-sm font-bold text-emerald-800 mt-0.5">
                <MoneyDisplay amount={batch.state.data.data.source_credit_idr} size="sm" variant="credit" />
              </p>
            </div>
            <div className="p-3.5 rounded-xl border border-slate-200 bg-white">
              <span className="text-[11px] text-slate-500 font-medium">Baris Diterima</span>
              <p className="font-mono text-base font-bold text-emerald-700 mt-0.5">
                {batch.state.data.data.accepted_rows.toLocaleString("id-ID")}
              </p>
            </div>
            <div className="p-3.5 rounded-xl border border-slate-200 bg-white">
              <span className="text-[11px] text-slate-500 font-medium">Karantina</span>
              <p className="font-mono text-base font-bold text-rose-700 mt-0.5">
                {batch.state.data.data.quarantined_rows.toLocaleString("id-ID")}
              </p>
            </div>
            <div className="p-3.5 rounded-xl border border-slate-200 bg-white">
              <span className="text-[11px] text-slate-500 font-medium">Sudah Diimpor</span>
              <p className="font-mono text-base font-bold text-slate-700 mt-0.5">
                {batch.state.data.data.committed_rows.toLocaleString("id-ID")}
              </p>
            </div>
          </div>

          {/* Customer Reconciliation Table */}
          <div className="space-y-3">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
              <div>
                <h4 className="text-xs font-bold text-slate-900 uppercase tracking-wider">
                  Daftar Rekonsiliasi Per Sheet Pelanggan ({identities.length})
                </h4>
                <p className="text-xs text-slate-500">
                  {plannedCustomerCount} pelanggan baru akan dibuat otomatis saat commit. Centang pelanggan yang ingin Anda impor ke ledger sekarang.
                </p>
              </div>
            </div>

            <div className="rounded-xl border border-slate-200 overflow-x-auto shadow-2xs">
              <table className="w-full text-left text-xs border-collapse">
                <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 font-semibold uppercase text-[11px]">
                  <tr>
                    <th className="p-3 pl-4 w-10 text-center">Pilih</th>
                    <th className="p-3">Sheet</th>
                    <th className="p-3">Pelanggan</th>
                    <th className="p-3 text-right">Debit Sumber</th>
                    <th className="p-3 text-right">Kredit Sumber</th>
                    <th className="p-3 text-right font-bold">Net Saldo</th>
                    <th className="p-3 text-right text-emerald-700">Baris Valid</th>
                    <th className="p-3 text-right text-rose-700">Karantina</th>
                    <th className="p-3">Status Cocok</th>
                    <th className="p-3 pr-4 text-center">Tinjau</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 bg-white">
                  {identities.map((identity) => (
                    <CustomerIdentityRow
                      key={identity.source_sheet}
                      batchId={id}
                      identity={identity}
                      csrf={csrf}
                      selected={selectedSheets.includes(identity.source_sheet)}
                      selectable={canCommitIdentity(identity)}
                      onSelected={(chk) => toggleCustomer(identity.source_sheet, chk)}
                      onChanged={() => {
                        setRefresh((val) => val + 1);
                        onChanged();
                      }}
                    />
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Selected Customer Commit Card */}
          {selectedCustomers.length > 0 && (
            <Card className="border-emerald-300 bg-emerald-50/30 p-6 shadow-sm space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h4 className="text-sm font-bold text-emerald-950">
                    Konfirmasi Commit Pelanggan Terpilih
                  </h4>
                  <p className="text-xs text-slate-600 mt-0.5">
                    <strong>{selectedCustomers.length} pelanggan</strong> dipilih •{" "}
                    <strong>{selectedValidRows.toLocaleString("id-ID")} baris transaksi</strong> akan resmi dimasukkan ke ledger.
                  </p>
                </div>
              </div>

              {/* Variance Visual Summary */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 p-4 rounded-xl bg-white border border-emerald-200 text-xs">
                <div>
                  <span className="text-slate-500">Net Sumber Belum Diimpor:</span>
                  <div className="mt-1">
                    <MoneyDisplay
                      amount={(selectedPendingDebit - selectedPendingCredit).toString()}
                      size="md"
                    />
                  </div>
                </div>

                <div>
                  <span className="text-slate-500">Net Baris Valid:</span>
                  <div className="mt-1">
                    <MoneyDisplay
                      amount={(selectedValidDebit - selectedValidCredit).toString()}
                      size="md"
                    />
                  </div>
                </div>

                <div>
                  <span className="text-slate-500">Variance Terhitung:</span>
                  <div className="mt-1 font-bold">
                    {selectedUncomputableRows > 0 ? (
                      <span className="text-amber-800">Perlu Rekonsiliasi Owner</span>
                    ) : (
                      <MoneyDisplay
                        amount={selectedKnownVariance.toString()}
                        size="md"
                        variant={selectedKnownVariance === 0n ? "success" : "danger"}
                      />
                    )}
                  </div>
                </div>
              </div>

              {varianceRequired && (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div className="space-y-1">
                    <label className="text-xs font-semibold text-slate-800" htmlFor="variance-idr">
                      Variance IDR (Wajib)
                    </label>
                    <Input
                      id="variance-idr"
                      value={variance}
                      onChange={(e) => setVariance(e.target.value)}
                      placeholder={selectedKnownVariance.toString()}
                      className="text-xs font-mono font-bold"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs font-semibold text-slate-800" htmlFor="variance-reason">
                      Alasan Selisih / Catatan Rekonsiliasi (Wajib)
                    </label>
                    <Input
                      id="variance-reason"
                      value={varianceReason}
                      onChange={(e) => setVarianceReason(e.target.value)}
                      maxLength={1000}
                      placeholder="Jelaskan alasan selisih untuk audit…"
                      className="text-xs"
                    />
                  </div>
                </div>
              )}

              <div className="flex items-center justify-end gap-2 pt-2">
                <Button
                  onClick={commit}
                  disabled={committing || selectedCustomers.some((i) => !canCommitIdentity(i))}
                  className="gap-2 font-semibold"
                >
                  <CheckCircle2 className="h-4 w-4" />
                  <span>{committing ? "Menyimpan ke Ledger…" : `Commit ${selectedValidRows.toLocaleString("id-ID")} Baris`}</span>
                </Button>
              </div>
            </Card>
          )}

          {commitError && (
            <div className="p-3.5 rounded-xl border border-slate-200 bg-slate-50 text-xs text-slate-800">
              {commitError}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function CustomerIdentityRow({
  identity,
  batchId,
  csrf,
  selected,
  selectable,
  onSelected,
  onChanged,
}: {
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
  const netSource = BigInt(identity.source_debit_idr) - BigInt(identity.source_credit_idr);

  return (
    <>
      <tr className="hover:bg-slate-50/70 transition-colors">
        <td className="p-3 pl-4 text-center">
          <input
            type="checkbox"
            checked={selected}
            disabled={!selectable}
            onChange={(e) => onSelected(e.target.checked)}
            className="h-4 w-4 rounded border-slate-300 text-emerald-800 focus:ring-emerald-700"
          />
        </td>

        <td className="p-3 font-mono font-medium text-slate-800">{identity.source_sheet}</td>

        <td className="p-3">
          <div className="font-semibold text-slate-900">{identity.source_name || "Nama Belum Terdeteksi"}</div>
          {identity.source_code && (
            <span className="text-[10px] font-mono text-slate-500">Kode: {identity.source_code}</span>
          )}
        </td>

        <td className="p-3 text-right font-mono text-slate-700">
          {formatRupiah(identity.source_debit_idr)}
        </td>

        <td className="p-3 text-right font-mono text-slate-700">
          {formatRupiah(identity.source_credit_idr)}
        </td>

        <td className="p-3 text-right font-mono font-bold text-slate-900">
          {formatRupiah(netSource.toString())}
        </td>

        <td className="p-3 text-right font-mono font-bold text-emerald-700">
          {identity.valid_rows.toLocaleString("id-ID")}
        </td>

        <td className="p-3 text-right font-mono font-bold text-rose-700">
          {exceptionRows.toLocaleString("id-ID")}
        </td>

        <td className="p-3">
          <Badge
            variant={
              identity.resolution === "will_create"
                ? "info"
                : identity.resolution.startsWith("matched")
                ? "success"
                : "warning"
            }
            className="text-[10px]"
          >
            {identity.resolution === "will_create"
              ? "Pelanggan Baru"
              : identity.resolution === "matched_code"
              ? "Cocok Kode"
              : identity.resolution === "matched_name"
              ? "Cocok Nama"
              : identity.resolution}
          </Badge>
        </td>

        <td className="p-3 pr-4 text-center">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setOpen(true)}
            className="h-7 text-xs px-2.5 gap-1"
          >
            <span>Buka</span>
            <ExternalLink className="h-3 w-3 text-slate-400" />
          </Button>
        </td>
      </tr>

      <Dialog
        open={open}
        onOpenChange={setOpen}
        title={`Rincian Sheet: ${identity.source_sheet} (${identity.source_name || "Tanpa Nama"})`}
        maxWidth="max-w-4xl"
      >
        <CustomerRowsDialog identity={identity} batchId={batchId} csrf={csrf} onChanged={onChanged} />
      </Dialog>
    </>
  );
}

function CustomerRowsDialog({
  identity,
  batchId,
  csrf,
  onChanged,
}: {
  identity: Schema["ImportCustomerIdentityPreview"];
  batchId: string;
  csrf: string;
  onChanged: () => void;
}) {
  const [cursorStack, setCursorStack] = useState<string[]>([]);
  const [reviewStatus, setReviewStatus] = useState(identity.valid_rows > 0 ? "valid" : "quarantined");
  const [refresh, setRefresh] = useState(0);
  const cursor = cursorStack.at(-1);

  const loadRows = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/import-batches/{id}/rows", {
          params: {
            path: { id: batchId },
            query: {
              ...(cursor ? { cursor } : {}),
              limit: 50,
              sheet_name: identity.source_sheet,
              review_status: reviewStatus || undefined,
            },
          },
          signal,
        }),
      ),
    [batchId, cursor, identity.source_sheet, refresh, reviewStatus],
  );

  const rows = useResource(loadRows);

  return (
    <div className="space-y-4">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3 bg-slate-50 rounded-xl border border-slate-200 text-xs">
        <div className="flex items-center gap-3">
          <span>Valid: <strong className="text-emerald-700">{identity.valid_rows}</strong></span>
          <span>•</span>
          <span>Karantina: <strong className="text-rose-700">{identity.quarantined_rows}</strong></span>
          <span>•</span>
          <span>Sudah Diimpor: <strong className="text-slate-700">{identity.committed_rows}</strong></span>
        </div>

        <select
          value={reviewStatus}
          onChange={(e) => {
            setReviewStatus(e.target.value);
            setCursorStack([]);
          }}
          className="h-8 rounded-lg border border-slate-300 bg-white px-2.5 text-xs text-slate-800"
        >
          <option value="">Semua Baris Sheet</option>
          <option value="valid">Semua Baris Valid</option>
          <option value="candidate">Kandidat</option>
          <option value="accepted">Diterima</option>
          <option value="quarantined">Karantina</option>
          <option value="excluded">Dikecualikan</option>
          <option value="committed">Sudah Diimpor</option>
        </select>
      </div>

      {rows.state.status === "loading" && (
        <div className="p-8 text-center text-xs text-slate-500">Memuat baris sheet…</div>
      )}

      {rows.state.status === "error" && (
        <ErrorNotice message={rows.state.error.message} retry={rows.reload} />
      )}

      {rows.state.status === "ready" && rows.state.data.data.length === 0 && (
        <div className="p-8 text-center text-xs text-slate-500">
          Tidak ada baris transaksi pada filter ini.
        </div>
      )}

      {rows.state.status === "ready" && rows.state.data.data.length > 0 && (
        <div className="rounded-xl border border-slate-200 overflow-x-auto shadow-2xs">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 font-semibold uppercase text-[10px]">
              <tr>
                <th className="p-2.5 pl-4">Baris</th>
                <th className="p-2.5 text-right">Debit</th>
                <th className="p-2.5 text-right">Kredit</th>
                <th className="p-2.5">Status</th>
                <th className="p-2.5 pr-4 text-center">Tinjau</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 bg-white">
              {rows.state.data.data.map((row) => (
                <ImportRowCard
                  key={row.id}
                  batchId={batchId}
                  row={row}
                  csrf={csrf}
                  onSaved={() => {
                    setRefresh((val) => val + 1);
                    onChanged();
                  }}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

const issueLabels: Record<string, string> = {
  duplicate_source_row: "Duplikat Baris",
  invalid_debit_credit: "Debit/Kredit 0",
  invalid_or_missing_date: "Tanggal Kosong/Salah",
  invalid_amount: "Nominal Salah",
  mixed_debit_credit: "Debit & Kredit Terisi",
  quantity_price_mismatch: "Qty × Harga Beda",
  future_business_date: "Tanggal Masa Depan",
  cell_error: "Cell Error",
  formula_customer_identity: "Identitas Formula",
};

function ImportRowCard({
  batchId,
  row,
  csrf,
  onSaved,
}: {
  batchId: string;
  row: Schema["ImportRow"];
  csrf: string;
  onSaved: () => void;
}) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [action, setAction] = useState<Schema["ImportReviewAction"]>(
    row.review_status === "quarantined" ? "override" : "accept",
  );
  const [customerId, setCustomerId] = useState(row.mapped_customer_id ?? "");
  const [customerSearch, setCustomerSearch] = useState("");
  const [reason, setReason] = useState("");
  const [note, setNote] = useState(row.review_note ?? "");
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);

  const loadCustomers = useCallback(
    async (signal: AbortSignal) =>
      readResult(
        await api.GET("/api/v1/customers", {
          params: { query: { search: customerSearch, limit: 25 } },
          signal,
        }),
      ),
    [customerSearch],
  );
  const customers = useResource(loadCustomers);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    setPending(true);
    try {
      const result = await api.PATCH("/api/v1/import-batches/{id}/rows/{row_id}", {
        params: { path: { id: batchId, row_id: row.id }, header: { "X-CSRF-Token": csrf } },
        body: {
          expected_version: row.version,
          action,
          mapped_customer_id: customerId || undefined,
          review_note: note || undefined,
          reason,
        },
      });
      readResult(result);
      setDialogOpen(false);
      onSaved();
    } catch (err) {
      setError(failure(err).message);
    } finally {
      setPending(false);
    }
  }

  const parsed = asRecord(row.parsed_values);
  const rowIssues = Array.isArray(row.issues)
    ? (row.issues as string[]).filter((i) => typeof i === "string" && i !== "transaction_data_issue")
    : [];

  return (
    <>
      <tr className="hover:bg-slate-50/70">
        <td className="p-2.5 pl-4 font-mono text-slate-700">
          Baris {row.row_number}
          <span className="block text-[10px] text-slate-400">
            {String(parsed.effective_date ?? "Tgl belum terdeteksi")}
          </span>
        </td>

        <td className="p-2.5 text-right font-mono font-bold text-amber-900">
          {displayMoney(parsed.debit_idr)}
        </td>

        <td className="p-2.5 text-right font-mono font-bold text-emerald-800">
          {displayMoney(parsed.credit_idr)}
        </td>

        <td className="p-2.5">
          <div className="flex flex-col gap-1 items-start">
            <Badge
              variant={
                row.review_status === "accepted" || row.review_status === "committed"
                  ? "success"
                  : row.review_status === "quarantined"
                  ? "destructive"
                  : "secondary"
              }
              className="text-[10px]"
            >
              {row.review_status}
            </Badge>
            {rowIssues.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {rowIssues.map((issue) => (
                  <span
                    key={issue}
                    className="text-[9px] font-medium text-rose-700 bg-rose-50 border border-rose-200 rounded px-1"
                  >
                    {issueLabels[issue] ?? issue}
                  </span>
                ))}
              </div>
            )}
          </div>
        </td>

        <td className="p-2.5 pr-4 text-center">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDialogOpen(true)}
            className="h-6 text-[11px] px-2"
          >
            Tinjau
          </Button>
        </td>
      </tr>

      <Dialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        title={`Telaah Baris ${row.row_number} (${row.sheet_name})`}
      >
        <form onSubmit={save} className="space-y-4">
          {rowIssues.length > 0 && (
            <div className="p-2.5 rounded-lg bg-rose-50 border border-rose-200 text-xs text-rose-800 space-y-1">
              <span className="font-semibold block text-[11px]">Isu / Alasan Karantina:</span>
              <ul className="list-disc list-inside text-[11px] space-y-0.5">
                {rowIssues.map((issue) => (
                  <li key={issue}>{issueLabels[issue] ?? issue}</li>
                ))}
              </ul>
            </div>
          )}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="row-action">
              Tindakan Telaah (Action)
            </label>
            <select
              id="row-action"
              value={action}
              onChange={(e) => setAction(e.target.value as Schema["ImportReviewAction"])}
              className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 text-xs text-slate-900"
            >
              <option value="accept">Terima (Valid)</option>
              <option value="quarantine">Karantina (Perlu Ditinjau)</option>
              <option value="exclude">Kecualikan dari Import</option>
            </select>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="cust-search">
              Hubungkan ke Pelanggan
            </label>
            <Input
              id="cust-search"
              value={customerSearch}
              onChange={(e) => setCustomerSearch(e.target.value)}
              placeholder="Cari nama atau kode pelanggan tujuan…"
              className="text-xs"
            />
            <select
              value={customerId}
              onChange={(e) => setCustomerId(e.target.value)}
              className="w-full h-9 rounded-lg border border-slate-300 bg-white px-3 text-xs text-slate-900 mt-1"
            >
              <option value="">-- Tetap Gunakan Pelanggan Sumber --</option>
              {customers.state.status === "ready" &&
                customers.state.data.data.items.map((item) => {
                  const cust = "customer" in item ? item.customer : item;
                  return (
                    <option key={cust.id} value={cust.id}>
                      {cust.name} {cust.code ? `(${cust.code})` : ""}
                    </option>
                  );
                })}
            </select>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700" htmlFor="row-reason">
              Alasan Tindakan (Audit) <span className="text-rose-600">*</span>
            </label>
            <Input
              id="row-reason"
              required
              maxLength={1000}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="contoh: Nilai telah diverifikasi sesuai nota fisik…"
              className="text-xs"
            />
          </div>

          {error && <ErrorNotice message={error} />}

          <div className="flex items-center justify-end gap-2 pt-2">
            <Button variant="outline" size="sm" onClick={() => setDialogOpen(false)} disabled={pending}>
              Batal
            </Button>
            <Button type="submit" size="sm" disabled={pending} className="font-semibold">
              {pending ? "Menyimpan…" : "Simpan Perubahan"}
            </Button>
          </div>
        </form>
      </Dialog>
    </>
  );
}

function formatRupiah(value: string): string {
  if (!/^-?\d+$/.test(value)) return `Rp ${value}`;
  return new Intl.NumberFormat("id-ID", {
    style: "currency",
    currency: "IDR",
    maximumFractionDigits: 0,
  }).format(BigInt(value));
}

function displayMoney(value: unknown): string {
  if (value === null || value === undefined || value === "") return "—";
  return formatRupiah(String(value));
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}
