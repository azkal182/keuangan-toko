use crate::*;
use gpui_kit::component::scroll::ScrollableElement;

impl Desktop {
    pub(crate) fn render_user_management_view(&self, cx: &mut Context<Self>) -> Div {
        let users = match &self.users_result {
            Some(Ok(users)) => users.clone(),
            Some(Err(error)) => {
                return div()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
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
                    .p(px(24.0))
                    .child(render_loading_state("Memuat daftar pengguna..."));
            }
        };
        let current_uid = self
            .session
            .as_ref()
            .map(|s| s.id.clone())
            .unwrap_or_default();

        let total_users = users.len();
        let total_owners = users.iter().filter(|u| u.role == UserRole::Owner).count();
        let total_operators = users
            .iter()
            .filter(|u| u.role == UserRole::Operator)
            .count();
        let total_active = users.iter().filter(|u| u.is_active).count();

        let columns = vec![
            GridColumn {
                title: "USERNAME",
                width_px: 185.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "PERAN (ROLE)",
                width_px: 155.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "STATUS AKUN",
                width_px: 115.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "KEWAJIBAN SANDI",
                width_px: 150.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "TANGGAL DIBUAT",
                width_px: 135.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "AKSI KELOLA AKUN",
                width_px: 260.0,
                align: ColumnAlign::Center,
            },
        ];
        let grid_width = 34.0 + columns.iter().map(|column| column.width_px).sum::<f32>();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .overflow_hidden()
            // CONTEXTUAL COMMAND BAR: HANYA AKSI MANAJEMEN PENGGUNA
            .child(
                render_toolbar_container()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                    .child("Manajemen Akun & Otorisasi Pengguna (Owner Only)"),
                            )
                            .child(tool_separator())
                            .child(
                                primary_action_button("+ Tambah Pengguna Baru", "")
                                    .id("tool-user-add")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_new_user_modal();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(Theme::TEXT_MUTED))
                            .child("Kebijakan: Kata Sandi Sementara Min. 15 Karakter • Minimal 1 Owner Aktif"),
                    ),
            )
            // WORKSPACE BODY
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .p(px(10.0))
                    .gap(px(8.0))
                    .overflow_hidden()
                    // 4 KPI Boxes matching Web UserManagementPage
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(self.native_summary_box(
                                "TOTAL PENGGUNA",
                                format!("{} Akun", total_users),
                                "Terdaftar dalam sistem toko",
                                Theme::TEXT_PRIMARY,
                                Theme::BORDER_STRONG,
                            ))
                            .child(self.native_summary_box(
                                "PERAN PEMILIK (OWNER)",
                                format!("{} Akun", total_owners),
                                "Hak akses administratif penuh",
                                Theme::PRIMARY,
                                Theme::PRIMARY_BORDER,
                            ))
                            .child(self.native_summary_box(
                                "OPERATOR KASIR",
                                format!("{} Akun", total_operators),
                                "Penjualan kredit & pencatatan bayar",
                                Theme::TEXT_PRIMARY,
                                Theme::BORDER_STRONG,
                            ))
                            .child(self.native_summary_box(
                                "STATUS KEAKTIFAN",
                                format!("{} Aktif", total_active),
                                "Akun yang memiliki izin masuk",
                                Theme::BADGE_CREDIT_TEXT,
                                Theme::BADGE_CREDIT_BORDER,
                            )),
                    )
                    // DBGrid Users
                    .child(
                        div()
                            .flex_1()
                            .bg(rgb(Theme::BG_CARD))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_STRONG))
                            .rounded(px(3.0))
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div().id("accounts-horizontal").flex_1().min_w_0().overflow_x_scrollbar().child(
                                    div().w(px(grid_width)).h_full().flex().flex_col()
                            .child(render_grid_header(&columns))
                            .child(
                                div()
                                    .id("user-mgr-scroll")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .children(
                                    users
                                        .into_iter()
                                        .enumerate()
                                        .map(|(idx, u)| {
                                            let bg_color = if idx % 2 == 1 {
                                                Theme::BG_ROW_ALT
                                            } else {
                                                Theme::BG_CARD
                                            };
                                            let is_self = u.id == current_uid;
                                            let u_edit = u.clone();
                                            let u_reset = u.clone();
                                            let u_toggle = u.clone();
                                            let uname_label = if is_self {
                                                format!("{} (Akun Anda)", u.username)
                                            } else {
                                                u.username.clone()
                                            };
                                            let date_str =
                                                u.created_at.format("%d %b %Y").to_string();

                                            div()
                                                .id(("u-row", idx))
                                                .h(px(28.0))
                                                .bg(rgb(bg_color))
                                                .border_b_1()
                                                .border_color(rgb(Theme::GRID_LINE))
                                                .flex()
                                                .items_center()
                                                .child(grid_row_indicator(idx + 1, is_self))
                                                .child(grid_cell_styled(
                                                    uname_label,
                                                    185.0,
                                                    ColumnAlign::Left,
                                                    Theme::TEXT_PRIMARY,
                                                    FontWeight::BOLD,
                                                    None,
                                                ))
                                                .child(match u.role {
                                                    UserRole::Owner => grid_badge(
                                                        "Pemilik (Owner)",
                                                        Theme::BADGE_CREDIT_TEXT,
                                                        Theme::BADGE_CREDIT_BG,
                                                        Theme::BADGE_CREDIT_BORDER,
                                                        155.0,
                                                    ),
                                                    UserRole::Operator => grid_badge(
                                                        "Operator Kasir",
                                                        Theme::PRIMARY,
                                                        Theme::PRIMARY_LIGHT,
                                                        Theme::PRIMARY_BORDER,
                                                        155.0,
                                                    ),
                                                    UserRole::Viewer => grid_badge(
                                                        "Pembaca (Viewer)",
                                                        Theme::TEXT_MUTED,
                                                        0xF1F5F9,
                                                        0xCBD5E1,
                                                        155.0,
                                                    ),
                                                })
                                                .child(if u.is_active {
                                                    grid_badge(
                                                        "Aktif",
                                                        Theme::BADGE_CREDIT_TEXT,
                                                        Theme::BADGE_CREDIT_BG,
                                                        Theme::BADGE_CREDIT_BORDER,
                                                        115.0,
                                                    )
                                                } else {
                                                    grid_badge(
                                                        "Nonaktif",
                                                        Theme::TEXT_MUTED,
                                                        0xF1F5F9,
                                                        0xCBD5E1,
                                                        115.0,
                                                    )
                                                })
                                                .child(if u.must_change_password {
                                                    grid_badge(
                                                        "Wajib Ganti Sandi",
                                                        Theme::BADGE_WARNING_TEXT,
                                                        Theme::BADGE_WARNING_BG,
                                                        Theme::BADGE_WARNING_BORDER,
                                                        150.0,
                                                    )
                                                } else {
                                                    grid_cell(
                                                        "Terverifikasi",
                                                        150.0,
                                                        ColumnAlign::Center,
                                                        false,
                                                    )
                                                })
                                                .child(grid_cell(
                                                    date_str,
                                                    135.0,
                                                    ColumnAlign::Center,
                                                    false,
                                                ))
                                                .child(
                                                    div()
                                                        .w(px(260.0))
                                                        .h_full()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .gap(px(5.0))
                                                        .child(
                                                            div()
                                                                .id(("edit-u-btn", idx))
                                                                .px(px(7.0))
                                                                .py(px(1.0))
                                                                .bg(rgb(Theme::BG_CARD))
                                                                .border_1()
                                                                .border_color(rgb(
                                                                    Theme::BORDER_STRONG,
                                                                ))
                                                                .rounded(px(2.0))
                                                                .cursor_pointer()
                                                                .hover(|d| {
                                                                    d.bg(rgb(Theme::BG_ROW_HOVER))
                                                                })
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        this.open_edit_user_modal(
                                                                            &u_edit,
                                                                        );
                                                                        cx.notify();
                                                                    },
                                                                ))
                                                                .child(
                                                                    div()
                                                                        .text_size(px(10.0))
                                                                        .font_weight(
                                                                            FontWeight::SEMIBOLD,
                                                                        )
                                                                        .text_color(rgb(
                                                                            Theme::TEXT_PRIMARY,
                                                                        ))
                                                                        .child("Ubah"),
                                                                ),
                                                        )
                                                        .when(!is_self && u.is_active, |d| {
                                                            d.child(
                                                                div()
                                                                    .id(("rst-u-btn", idx))
                                                                    .px(px(7.0))
                                                                    .py(px(1.0))
                                                                    .bg(rgb(
                                                                        Theme::BADGE_WARNING_BG,
                                                                    ))
                                                                    .border_1()
                                                                    .border_color(rgb(
                                                                        Theme::BADGE_WARNING_BORDER,
                                                                    ))
                                                                    .rounded(px(2.0))
                                                                    .cursor_pointer()
                                                                    .on_click(cx.listener(
                                                                        move |this, _, _, cx| {
                                                                            this.open_reset_user_password_modal(&u_reset);
                                                                            cx.notify();
                                                                        },
                                                                    ))
                                                                    .child(
                                                                        div()
                                                                            .text_size(px(10.0))
                                                                            .font_weight(
                                                                                FontWeight::BOLD,
                                                                            )
                                                                            .text_color(rgb(
                                                                                Theme::BADGE_WARNING_TEXT,
                                                                            ))
                                                                            .child("Reset Sandi"),
                                                                    ),
                                                            )
                                                        })
                                                        .when(!is_self, |d| {
                                                            let is_act = u.is_active;
                                                            d.child(
                                                                div()
                                                                    .id(("tgl-u-btn", idx))
                                                                    .px(px(7.0))
                                                                    .py(px(1.0))
                                                                    .bg(rgb(if is_act {
                                                                        Theme::BADGE_DEBIT_BG
                                                                    } else {
                                                                        Theme::BADGE_CREDIT_BG
                                                                    }))
                                                                    .border_1()
                                                                    .border_color(rgb(if is_act {
                                                                        Theme::BADGE_DEBIT_BORDER
                                                                    } else {
                                                                        Theme::BADGE_CREDIT_BORDER
                                                                    }))
                                                                    .rounded(px(2.0))
                                                                    .cursor_pointer()
                                                                    .on_click(cx.listener(
                                                                        move |this, _, _, cx| {
                                                                            this.toggle_user_active(u_toggle.clone(), cx);
                                                                        },
                                                                    ))
                                                                    .child(
                                                                        div()
                                                                            .text_size(px(10.0))
                                                                            .font_weight(
                                                                                FontWeight::BOLD,
                                                                            )
                                                                            .text_color(rgb(
                                                                                if is_act {
                                                                                    Theme::BADGE_DEBIT_TEXT
                                                                                } else {
                                                                                    Theme::BADGE_CREDIT_TEXT
                                                                                },
                                                                            ))
                                                                            .child(if is_act {
                                                                                "Nonaktifkan"
                                                                            } else {
                                                                                "Aktifkan"
                                                                            }),
                                                                    ),
                                                            )
                                                        }),
                                                )
                                                .into_any_element()
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                            )
                                )
                            ),
                    ),
            )
    }
}
