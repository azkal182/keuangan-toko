import { useState, useRef, useEffect, type ReactNode } from "react";
import { Link, NavLink, useLocation } from "react-router-dom";
import {
  Users,
  Clock,
  FileSpreadsheet,
  LogOut,
  Shield,
  User,
  Store,
  KeyRound,
  UserCog,
  ChevronRight,
  ChevronDown,
  Home,
} from "lucide-react";
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
  passwordChangeRequired = false,
}: {
  session: Schema["SessionDetails"];
  onLogout: () => void;
  children: ReactNode;
  actionButton?: ReactNode;
  passwordChangeRequired?: boolean;
}) {
  const location = useLocation();
  const canWrite = session.user.role !== "viewer";

  const getPageInfo = () => {
    if (location.pathname.startsWith("/reports/aging")) {
      return {
        title: "Laporan Umur Piutang",
        subtitle: "Analisis distribusi saldo piutang terbuka berdasarkan kelompok umur kalender usaha.",
        breadcrumbs: [{ label: "Laporan", href: "/reports/aging" }, { label: "Umur Piutang" }],
      };
    }
    if (location.pathname.startsWith("/imports")) {
      return {
        title: "Import Workbook Historis",
        subtitle: "Staging, review, rekonsiliasi sheet pelanggan, dan commit data historis Excel.",
        breadcrumbs: [{ label: "Import", href: "/imports" }, { label: "Workbook" }],
      };
    }
    if (location.pathname.startsWith("/settings/users")) {
      return {
        title: "Manajemen Pengguna",
        subtitle: "Kelola akun pengguna, peran otorisasi (Pemilik/Operator/Pembaca), dan reset kata sandi.",
        breadcrumbs: [{ label: "Pengaturan", href: "/settings/users" }, { label: "Pengguna" }],
      };
    }
    if (location.pathname.startsWith("/settings/account")) {
      return {
        title: "Pengaturan Akun & Kata Sandi",
        subtitle: "Perbarui kata sandi akun Anda untuk menjaga keamanan akses sistem.",
        breadcrumbs: [{ label: "Pengaturan", href: "/settings/account" }, { label: "Akun" }],
      };
    }
    if (location.pathname.startsWith("/customers/")) {
      return {
        title: "Detail Pelanggan",
        subtitle: "Rincian buku piutang, faktur penjualan kredit, dan riwayat pembayaran pelanggan.",
        breadcrumbs: [{ label: "Pelanggan", href: "/customers" }, { label: "Detail" }],
      };
    }
    return {
      title: "Daftar Pelanggan",
      subtitle: "Kelola direktori pelanggan, saldo piutang terutang, dan status penagihan toko.",
      breadcrumbs: [{ label: "Pelanggan", href: "/customers" }],
    };
  };

  const pageInfo = getPageInfo();

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col antialiased">
      {/* Top Header */}
      <header className="sticky top-0 z-30 border-b border-slate-200/90 bg-white/95 backdrop-blur-md px-4 sm:px-8 py-2.5 transition-all">
        <div className="max-w-7xl mx-auto flex items-center justify-between gap-4">
          {/* Brand */}
          <Link to="/customers" className="flex items-center gap-3 group">
            <div className="flex items-center justify-center h-10 w-10 rounded-xl bg-gradient-to-br from-[#0f4c3a] to-[#092e23] text-white shadow-sm ring-1 ring-emerald-950/20 group-hover:scale-105 transition-transform">
              <Store className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-bold text-slate-900 text-sm tracking-tight">Piutang Reza Putra</span>
                <span className="text-[10px] uppercase tracking-wider font-semibold px-1.5 py-0.5 rounded bg-emerald-50 text-emerald-800 border border-emerald-200">
                  Web
                </span>
              </div>
              <p className="text-[11px] text-slate-500 font-medium">Buku Piutang & Penjualan Kredit</p>
            </div>
          </Link>

          {/* Navigation Tabs (Desktop) */}
          <nav aria-label="Navigasi Utama" className="hidden md:flex items-center gap-1.5 bg-slate-100 p-1 rounded-xl border border-slate-200/80">
            {!passwordChangeRequired && (
              <>
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

                {session.user.role === "owner" && (
                  <NavLink
                    to="/settings/users"
                    className={({ isActive }) =>
                      `inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                        isActive
                          ? "bg-white text-emerald-950 shadow-2xs"
                          : "text-slate-600 hover:text-slate-900 hover:bg-white/50"
                      }`
                    }
                  >
                    <UserCog className="h-4 w-4 text-emerald-700" />
                    <span>Pengguna</span>
                  </NavLink>
                )}
              </>
            )}
          </nav>

          {/* Avatar Dropdown for Account & Logout */}
          <UserAvatarDropdown session={session} onLogout={onLogout} />
        </div>

        {/* Mobile Navigation bar: Spacious, only primary business modules */}
        {!passwordChangeRequired && (
          <nav aria-label="Navigasi Mobile" className="flex md:hidden items-center justify-around gap-2 pt-2.5 mt-2.5 border-t border-slate-200">
            <NavLink
              to="/customers"
              className={({ isActive }) =>
                `flex flex-1 items-center justify-center gap-1.5 py-2 rounded-xl text-xs font-semibold transition-all ${
                  isActive
                    ? "bg-emerald-50 text-emerald-900 border border-emerald-200 shadow-2xs"
                    : "text-slate-600 hover:bg-slate-100"
                }`
              }
            >
              <Users className="h-4 w-4 text-emerald-700" />
              <span>Pelanggan</span>
            </NavLink>

            <NavLink
              to="/reports/aging"
              className={({ isActive }) =>
                `flex flex-1 items-center justify-center gap-1.5 py-2 rounded-xl text-xs font-semibold transition-all ${
                  isActive
                    ? "bg-emerald-50 text-emerald-900 border border-emerald-200 shadow-2xs"
                    : "text-slate-600 hover:bg-slate-100"
                }`
              }
            >
              <Clock className="h-4 w-4 text-emerald-700" />
              <span>Aging</span>
            </NavLink>

            {canWrite && (
              <NavLink
                to="/imports"
                className={({ isActive }) =>
                  `flex flex-1 items-center justify-center gap-1.5 py-2 rounded-xl text-xs font-semibold transition-all ${
                    isActive
                      ? "bg-emerald-50 text-emerald-900 border border-emerald-200 shadow-2xs"
                      : "text-slate-600 hover:bg-slate-100"
                  }`
                }
              >
                <FileSpreadsheet className="h-4 w-4 text-emerald-700" />
                <span>Import</span>
              </NavLink>
            )}
          </nav>
        )}
      </header>

      {/* Main Container */}
      <main className="flex-1 max-w-7xl w-full mx-auto p-4 sm:p-8 space-y-6">
        {/* Context Breadcrumbs */}
        <div className="flex items-center gap-1.5 text-xs text-slate-500">
          <Link to="/customers" className="hover:text-emerald-800 flex items-center gap-1">
            <Home className="h-3 w-3" />
            <span>Beranda</span>
          </Link>
          {pageInfo.breadcrumbs.map((crumb, idx) => (
            <div key={idx} className="flex items-center gap-1.5">
              <ChevronRight className="h-3 w-3 text-slate-400" />
              {crumb.href ? (
                <Link to={crumb.href} className="hover:text-emerald-800">
                  {crumb.label}
                </Link>
              ) : (
                <span className="font-semibold text-slate-800">{crumb.label}</span>
              )}
            </div>
          ))}
        </div>

        {/* Page Top Context bar */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-slate-200/70">
          <div>
            <h1 className="text-xl sm:text-2xl font-bold tracking-tight text-slate-900">
              {pageInfo.title}
            </h1>
            <p className="text-xs sm:text-sm text-slate-500 mt-0.5">
              {pageInfo.subtitle}
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
          <span>&copy; {new Date().getFullYear()} Piutang Reza Putra — Sistem Pencatatan & Piutang Pelanggan</span>
          <span className="font-mono text-[11px] text-slate-400">PostgreSQL · Axum · React</span>
        </div>
      </footer>
    </div>
  );
}

function UserAvatarDropdown({
  session,
  onLogout,
}: {
  session: Schema["SessionDetails"];
  onLogout: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const location = useLocation();

  const roleConfig = roleLabels[session.user.role] ?? { label: session.user.role, variant: "secondary" as const };

  // Close dropdown on click outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    if (open) {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("keydown", handleKeyDown);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  // Close dropdown on route change
  useEffect(() => {
    setOpen(false);
  }, [location.pathname]);

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        type="button"
        aria-expanded={open}
        aria-haspopup="true"
        onClick={() => setOpen((prev) => !prev)}
        className={`flex items-center gap-2 p-1.5 sm:px-2.5 sm:py-1.5 rounded-xl transition-all border cursor-pointer select-none ${
          open
            ? "bg-emerald-50 border-emerald-300 ring-2 ring-emerald-600/20"
            : "bg-slate-50/80 border-slate-200 hover:bg-slate-100 hover:border-slate-300 shadow-2xs"
        }`}
      >
        <div className="flex items-center justify-center h-7 w-7 rounded-lg bg-gradient-to-br from-[#0f4c3a] to-[#0a3528] text-white font-bold text-xs shadow-2xs">
          {session.user.username.slice(0, 1).toUpperCase()}
        </div>
        <div className="hidden sm:flex flex-col items-start leading-none gap-0.5">
          <span className="text-xs font-bold text-slate-900">{session.user.username}</span>
          <Badge variant={roleConfig.variant} className="text-[10px] py-0 px-1.5 h-3.5 font-medium">
            {roleConfig.label}
          </Badge>
        </div>
        <ChevronDown
          className={`h-3.5 w-3.5 text-slate-400 transition-transform duration-150 ${
            open ? "rotate-180 text-emerald-800" : ""
          }`}
        />
      </button>

      {open && (
        <div className="absolute right-0 mt-2 w-64 rounded-2xl border border-slate-200 bg-white p-2 shadow-xl ring-1 ring-black/5 z-50 animate-in fade-in zoom-in-95 duration-100">
          {/* User profile overview */}
          <div className="p-3 border-b border-slate-100 bg-slate-50/80 rounded-xl mb-1.5">
            <div className="flex items-center gap-2.5">
              <div className="flex items-center justify-center h-9 w-9 rounded-xl bg-gradient-to-br from-emerald-800 to-emerald-950 text-white font-bold text-xs shadow-2xs">
                {session.user.username.slice(0, 1).toUpperCase()}
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-xs font-bold text-slate-900 truncate">{session.user.username}</span>
                <div className="flex items-center gap-1.5 mt-0.5">
                  <Badge variant={roleConfig.variant} className="text-[10px] py-0 px-1.5 h-3.5 font-medium">
                    {roleConfig.label}
                  </Badge>
                </div>
              </div>
            </div>
          </div>

          {/* Menu items */}
          <div className="space-y-0.5">
            <Link
              to="/settings/account"
              onClick={() => setOpen(false)}
              className={`flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                location.pathname.startsWith("/settings/account")
                  ? "bg-emerald-50 text-emerald-900 font-semibold"
                  : "text-slate-700 hover:bg-slate-100 hover:text-slate-900"
              }`}
            >
              <KeyRound className="h-4 w-4 text-emerald-700" />
              <span>Ganti Kata Sandi & Akun</span>
            </Link>

            {session.user.role === "owner" && (
              <Link
                to="/settings/users"
                onClick={() => setOpen(false)}
                className={`flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                  location.pathname.startsWith("/settings/users")
                    ? "bg-emerald-50 text-emerald-900 font-semibold"
                    : "text-slate-700 hover:bg-slate-100 hover:text-slate-900"
                }`}
              >
                <UserCog className="h-4 w-4 text-emerald-700" />
                <span>Manajemen Pengguna</span>
              </Link>
            )}
          </div>

          <div className="my-1.5 border-t border-slate-100" />

          <button
            type="button"
            disabled={loggingOut}
            onClick={() => {
              setLoggingOut(true);
              onLogout();
            }}
            className="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium text-rose-700 hover:bg-rose-50 hover:text-rose-900 transition-colors cursor-pointer disabled:opacity-50"
          >
            <LogOut className="h-4 w-4 text-rose-600" />
            <span>{loggingOut ? "Mengeluarkan sesi…" : "Keluar (Logout)"}</span>
          </button>
        </div>
      )}
    </div>
  );
}
