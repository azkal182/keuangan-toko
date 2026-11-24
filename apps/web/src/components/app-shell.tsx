import { useState, type ReactNode } from "react";
import { NavLink, useLocation } from "react-router-dom";
import { Users, Clock, FileSpreadsheet, LogOut, Shield, User, Store } from "lucide-react";
import type { Schema } from "../api";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";

const roleLabels: Record<Schema["UserRole"], { label: string; variant: "success" | "info" | "secondary" }> = {
  owner: { label: "Pemilik", variant: "success" },
  operator: { label: "Operator", variant: "info" },
  viewer: { label: "Pembaca", variant: "secondary" },
};

export function AppShell({
  session,
  onLogout,
  children,
  actionButton,
}: {
  session: Schema["SessionDetails"];
  onLogout: () => void;
  children: ReactNode;
  actionButton?: ReactNode;
}) {
  const location = useLocation();
  const [loggingOut, setLoggingOut] = useState(false);
  const canWrite = session.user.role !== "viewer";
  const roleConfig = roleLabels[session.user.role] ?? { label: session.user.role, variant: "secondary" as const };

  const getPageTitle = () => {
    if (location.pathname.startsWith("/reports/aging")) return "Laporan Umur Piutang";
    if (location.pathname.startsWith("/imports")) return "Import Workbook Historis";
    if (location.pathname.startsWith("/customers/")) return "Detail Pelanggan";
    return "Daftar Pelanggan";
  };

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col antialiased">
      {/* Top Header */}
      <header className="sticky top-0 z-30 border-b border-slate-200/90 bg-white/95 backdrop-blur-md px-4 sm:px-8 py-2.5 transition-all">
        <div className="max-w-7xl mx-auto flex items-center justify-between gap-4">
          {/* Brand */}
          <div className="flex items-center gap-3">
            <div className="flex items-center justify-center h-10 w-10 rounded-xl bg-gradient-to-br from-[#0f4c3a] to-[#092e23] text-white shadow-sm ring-1 ring-emerald-950/20">
              <Store className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-bold text-slate-900 text-sm tracking-tight">Keuangan Toko</span>
                <span className="text-[10px] uppercase tracking-wider font-semibold px-1.5 py-0.5 rounded bg-emerald-50 text-emerald-800 border border-emerald-200">
                  Web
                </span>
              </div>
              <p className="text-[11px] text-slate-500 font-medium">Buku Piutang & Penjualan Kredit</p>
            </div>
          </div>

          {/* Navigation Tabs (Desktop) */}
          <nav aria-label="Navigasi Utama" className="hidden md:flex items-center gap-1.5 bg-slate-100 p-1 rounded-xl border border-slate-200/80">
            <NavLink
              to="/customers"
              className={({ isActive }) =>
                `inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  isActive
                    ? "bg-white text-emerald-950 shadow-2xs"
                    : "text-slate-600 hover:text-slate-900 hover:bg-white/50"
                }`
              }
            >
              <Users className="h-4 w-4 text-emerald-700" />
              <span>Pelanggan</span>
            </NavLink>

            <NavLink
              to="/reports/aging"
              className={({ isActive }) =>
                `inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  isActive
                    ? "bg-white text-emerald-950 shadow-2xs"
                    : "text-slate-600 hover:text-slate-900 hover:bg-white/50"
                }`
              }
            >
              <Clock className="h-4 w-4 text-emerald-700" />
              <span>Umur Piutang</span>
            </NavLink>

            {canWrite && (
              <NavLink
                to="/imports"
                className={({ isActive }) =>
                  `inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                    isActive
                      ? "bg-white text-emerald-950 shadow-2xs"
                      : "text-slate-600 hover:text-slate-900 hover:bg-white/50"
                  }`
                }
              >
                <FileSpreadsheet className="h-4 w-4 text-emerald-700" />
                <span>Import Workbook</span>
              </NavLink>
            )}
          </nav>

          {/* Account Profile & Logout */}
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2.5 pl-3 border-l border-slate-200">
              <div className="flex items-center justify-center h-8 w-8 rounded-full bg-slate-100 text-slate-700 border border-slate-200">
                {session.user.role === "owner" ? <Shield className="h-4 w-4 text-emerald-800" /> : <User className="h-4 w-4" />}
              </div>
              <div className="hidden sm:flex flex-col items-start leading-none gap-1">
                <span className="text-xs font-semibold text-slate-900">{session.user.username}</span>
                <Badge variant={roleConfig.variant} className="text-[10px] py-0 px-1.5 h-4">
                  {roleConfig.label}
                </Badge>
              </div>
            </div>

            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setLoggingOut(true);
                onLogout();
              }}
              disabled={loggingOut}
              className="gap-1.5 text-xs text-slate-700 hover:text-rose-700 hover:border-rose-300 hover:bg-rose-50"
            >
              <LogOut className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">{loggingOut ? "Keluar…" : "Keluar"}</span>
            </Button>
          </div>
        </div>

        {/* Mobile Navigation bar */}
        <nav aria-label="Navigasi Mobile" className="flex md:hidden items-center justify-around gap-1 pt-2.5 mt-2.5 border-t border-slate-200">
          <NavLink
            to="/customers"
            className={({ isActive }) =>
              `flex flex-1 items-center justify-center gap-1.5 py-1.5 rounded-lg text-xs font-semibold ${
                isActive ? "bg-emerald-50 text-emerald-900 border border-emerald-200" : "text-slate-600"
              }`
            }
          >
            <Users className="h-3.5 w-3.5" />
            <span>Pelanggan</span>
          </NavLink>
          <NavLink
            to="/reports/aging"
            className={({ isActive }) =>
              `flex flex-1 items-center justify-center gap-1.5 py-1.5 rounded-lg text-xs font-semibold ${
                isActive ? "bg-emerald-50 text-emerald-900 border border-emerald-200" : "text-slate-600"
              }`
            }
          >
            <Clock className="h-3.5 w-3.5" />
            <span>Aging</span>
          </NavLink>
          {canWrite && (
            <NavLink
              to="/imports"
              className={({ isActive }) =>
                `flex flex-1 items-center justify-center gap-1.5 py-1.5 rounded-lg text-xs font-semibold ${
                  isActive ? "bg-emerald-50 text-emerald-900 border border-emerald-200" : "text-slate-600"
                }`
              }
            >
              <FileSpreadsheet className="h-3.5 w-3.5" />
              <span>Import</span>
            </NavLink>
          )}
        </nav>
      </header>

      {/* Main Container */}
      <main className="flex-1 max-w-7xl w-full mx-auto p-4 sm:p-8 space-y-6">
        {/* Page Top Context bar */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-slate-200/70">
          <div>
            <h1 className="text-xl sm:text-2xl font-bold tracking-tight text-slate-900">
              {getPageTitle()}
            </h1>
            <p className="text-xs sm:text-sm text-slate-500 mt-0.5">
              Kelola pencatatan piutang pelanggan, penjualan kredit, dan mutasi kas secara atomik.
            </p>
          </div>
          {actionButton && <div className="flex items-center gap-2">{actionButton}</div>}
        </div>

        {/* Content */}
        {children}
      </main>

      {/* Footer */}
      <footer className="mt-auto border-t border-slate-200 bg-white py-4 px-6 text-center text-xs text-slate-500">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-2">
          <span>&copy; {new Date().getFullYear()} Keuangan Toko — Sistem Pencatatan Keuangan & Piutang Toko</span>
          <span className="font-mono text-[11px] text-slate-400">PostgreSQL · Axum · React</span>
        </div>
      </footer>
    </div>
  );
}
