import { Button } from "./ui/button";

export function Brand() {
  return (
    <div className="brand">
      <span className="brand-mark" aria-hidden="true">KT</span>
      <span>Keuangan Toko</span>
    </div>
  );
}

export function ErrorNotice({ message, retry }: { message: string; retry?: () => void }) {
  return (
    <div className="notice-error" role="alert">
      <p>{message}</p>
      {retry && <Button variant="outline" onClick={retry}>Coba lagi</Button>}
    </div>
  );
}

