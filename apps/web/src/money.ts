const rupiah = new Intl.NumberFormat("id-ID", {
  style: "currency",
  currency: "IDR",
  maximumFractionDigits: 0,
});

export function formatRupiah(amount: string) {
  return rupiah.format(BigInt(amount));
}

export function formatQuantity(qty: string | null | undefined): string {
  if (!qty) return "—";
  const num = parseFloat(qty);
  if (isNaN(num)) return qty;
  // If integer, display without decimal places (e.g. "5.000" -> "5")
  if (Number.isInteger(num)) {
    return num.toLocaleString("id-ID");
  }
  // Otherwise format with up to 3 decimals using Indonesian comma
  return num.toLocaleString("id-ID", { maximumFractionDigits: 3 });
}
