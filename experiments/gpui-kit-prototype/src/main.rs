mod customers;
mod theme;
use customers::Customers;
use gpui_kit::component::{
    WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    label::Label,
    scroll::ScrollbarMode,
    table::{DataTable, TableDelegate, TableSelection, TableState},
};
use gpui_kit::*;

actions!(prototype, [FocusNext, FocusPrevious]);

struct Prototype {
    search: Entity<InputState>,
    table: Entity<TableState<Customers>>,
    show_error: bool,
    _subscription: Subscription,
}
impl Prototype {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Nama pelanggan contoh"));
        let table =
            cx.new(|cx| TableState::new(Customers::new(), window, cx).cell_selectable(true));
        let subscription =
            cx.subscribe_in(&search, window, |view: &mut Self, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = state.read(cx).value().to_string();
                    view.table.update(cx, |table, cx| {
                        table.delegate_mut().filter(&query);
                        table.set_selection(TableSelection::None, cx);
                        table.refresh(cx);
                    });
                    cx.notify();
                }
            });
        Self {
            search,
            table,
            show_error: false,
            _subscription: subscription,
        }
    }
}
impl Render for Prototype {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = self.table.read(cx).delegate().rows_count(cx) == 0;
        div()
            .on_action(cx.listener(|_, _: &FocusNext, window, cx| window.focus_next(cx)))
            .on_action(cx.listener(|_, _: &FocusPrevious, window, cx| window.focus_prev(cx)))
            .size_full()
            .flex()
            .flex_col()
            .gap(px(theme::SECTION_GAP))
            .p(px(theme::PAGE_PADDING))
            .child(Label::new("Keuangan Toko - Prototipe GPUI Kit"))
            .child(Label::new(
                "DATA CONTOH - Tidak menyimpan atau mengirim data",
            ))
            .child(Label::new("Cari pelanggan"))
            .child(Input::new(&self.search))
            .child(if self.show_error {
                Label::new("Kesalahan contoh: data tidak dapat dimuat.")
            } else if empty {
                Label::new("Tidak ada pelanggan yang cocok.")
            } else {
                Label::new(
                    "Pilih sel, gunakan tombol panah, dan coba urutkan atau ubah lebar kolom.",
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h(px(theme::TABLE_MIN_HEIGHT))
                    .child(
                        DataTable::new(&self.table)
                            .scrollbar_mode(ScrollbarMode::Always)
                            .accessibility_label("Daftar pelanggan contoh"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(theme::ACTION_GAP))
                    .child(
                        Button::new("dialog")
                            .primary()
                            .label("Uji dialog")
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog
                                        .title("Konfirmasi contoh")
                                        .child("Ini hanya pengujian. Tidak ada transaksi.")
                                });
                            }),
                    )
                    .child(
                        Button::new("error")
                            .label(if self.show_error {
                                "Tutup error contoh"
                            } else {
                                "Tampilkan error contoh"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_error = !this.show_error;
                                cx.notify();
                            })),
                    ),
            )
    }
}
fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        cx.bind_keys([
            KeyBinding::new("tab", FocusNext, None),
            KeyBinding::new("shift-tab", FocusPrevious, None),
        ]);
        let options = if std::env::args().any(|arg| arg == "--narrow") {
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(480.), px(700.)),
                    cx,
                ))),
                ..Default::default()
            }
        } else {
            WindowOptions::default()
        };
        open_window(options, cx, |window, cx| {
            cx.new(|cx| Prototype::new(window, cx))
        })
        .expect("Gagal membuka jendela");
    });
}
