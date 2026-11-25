const rupiah = new Intl.NumberFormat("id-ID", {
  style: "currency",
  currency: "IDR",
  maximumFractionDigits: 0,
});

export function formatRupiah(amount: string) {
  return rupiah.format(BigInt(amount));
}

export function formatAccountingRupiah(amount: string | bigint | number | undefined | null): string {
  if (amount === undefined || amount === null || amount === "") return "—";
  const str = String(amount);
  if (str.startsWith("-")) {
    const positive = str.slice(1);
    return `(${formatRupiah(positive)})`;
  }
  return formatRupiah(str);
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
