use crate::*;
use gpui_kit::base::Disableable;
use gpui_kit::component::{
    scroll::ScrollbarMode,
    table::{DataTable, TableSelection},
};

impl Desktop {
    pub(crate) fn render_user_management_view(&self, cx: &mut Context<Self>) -> Div {
        let users = match &self.users_result {
            Some(Ok(users)) => users.clone(),
            Some(Err(error)) => {
                return div()
                    .p(px(24.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(format!("Gagal memuat pengguna: {}", error.0))
                    .child(
                        Button::new("retry-users")
                            .outline()
                            .label("Coba lagi")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.store.invalidate_cache();
                                this.invalidate_aging();
                                cx.notify();
                            })),
                    );
            }
            None => {
                return div()
                    .p(px(24.))
                    .child(render_loading_state("Memuat daftar pengguna..."));
            }
        };
        let current_id = self
            .session
            .as_ref()
            .map(|s| s.id.clone())
            .unwrap_or_default();
        let total = users.len();
        let owners = users.iter().filter(|u| u.role == UserRole::Owner).count();
        let operators = users
            .iter()
            .filter(|u| u.role == UserRole::Operator)
            .count();
        let active = users.iter().filter(|u| u.is_active).count();
        self.account_table.update(cx, |table, cx| {
            if table
                .delegate_mut()
                .replace(users.clone(), current_id.clone())
            {
                table.set_selection(TableSelection::None, cx);
                table.refresh(cx);
            }
        });
        let selected = self
            .account_table
            .read(cx)
            .selected_row()
            .and_then(|row| self.account_table.read(cx).delegate().rows.get(row))
            .cloned();
        let editable = selected.is_some();
        let can_reset = selected
            .as_ref()
            .is_some_and(|u| u.id != current_id && u.is_active);
        let can_toggle = selected.as_ref().is_some_and(|u| u.id != current_id);
        let toggle_label = if selected.as_ref().is_some_and(|u| u.is_active) {
            "Nonaktifkan"
        } else {
            "Aktifkan"
        };
        let edit = selected.clone();
        let reset = selected.clone();
        let toggle = selected;
        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .overflow_hidden()
            .child(
                render_toolbar_container()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Manajemen Pengguna"),
                            )
                            .child(
                                Button::new("user-add")
                                    .primary()
                                    .label("Tambah pengguna")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_new_user_modal();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(Theme::TEXT_MUTED))
                            .child("Minimal satu Owner aktif"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .p(px(10.))
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(self.native_summary_box(
                                "TOTAL PENGGUNA",
                                format!("{total} Akun"),
                                "Terdaftar dalam sistem toko",
                                Theme::TEXT_PRIMARY,
                                Theme::BORDER_STRONG,
                            ))
                            .child(self.native_summary_box(
                                "PERAN PEMILIK (OWNER)",
                                format!("{owners} Akun"),
                                "Hak akses administratif penuh",
                                Theme::PRIMARY,
                                Theme::PRIMARY_BORDER,
                            ))
                            .child(self.native_summary_box(
                                "OPERATOR KASIR",
                                format!("{operators} Akun"),
                                "Penjualan kredit dan pembayaran",
                                Theme::TEXT_PRIMARY,
                                Theme::BORDER_STRONG,
                            ))
                            .child(self.native_summary_box(
                                "STATUS KEAKTIFAN",
                                format!("{active} Aktif"),
                                "Akun yang memiliki izin masuk",
                                Theme::BADGE_CREDIT_TEXT,
                                Theme::BADGE_CREDIT_BORDER,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(Theme::TEXT_MUTED))
                                    .child("Pilih pengguna untuk mengelola akun."),
                            )
                            .child(
                                Button::new("user-edit")
                                    .outline()
                                    .label("Ubah")
                                    .disabled(!editable)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(user) = &edit {
                                            this.open_edit_user_modal(user);
                                            cx.notify();
                                        }
                                    })),
                            )
                            .child(
                                Button::new("user-reset")
                                    .outline()
                                    .label("Reset sandi")
                                    .disabled(!can_reset)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(user) = &reset {
                                            this.open_reset_user_password_modal(user);
                                            cx.notify();
                                        }
                                    })),
                            )
                            .child(
                                Button::new("user-toggle")
                                    .outline()
                                    .label(toggle_label)
                                    .disabled(!can_toggle)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(user) = &toggle {
                                            this.toggle_user_active(user.clone(), cx);
                                        }
                                    })),
                            ),
                    )
                    .child(if users.is_empty() {
                        div()
                            .flex_1()
                            .child(render_empty_state("Belum ada akun pengguna."))
                            .into_any_element()
                    } else {
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h(px(150.))
                            .child(
                                DataTable::new(&self.account_table)
                                    .scrollbar_mode(ScrollbarMode::Always)
                                    .accessibility_label("Daftar akun pengguna"),
                            )
                            .into_any_element()
                    }),
            )
    }
}
