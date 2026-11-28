pub mod navigation;

use crate::{
    Desktop,
    store::{HttpApiAdapter, TokoStoreAdapter, mock_store::MockDataAdapter},
};
use gpui::{App, AppContext, Bounds, WindowBounds, WindowOptions, px, size};

/// Pilih sumber data: API nyata (default) atau mock lokal (`TOKO_DESKTOP_ADAPTER=mock`).
/// Base URL API dapat diubah lewat `TOKO_API_BASE_URL`.
pub(crate) fn build_store_from_env() -> Box<dyn TokoStoreAdapter> {
    let mode = std::env::var("TOKO_DESKTOP_ADAPTER")
        .unwrap_or_default()
        .to_lowercase();
    if mode == "mock" {
        return Box::new(MockDataAdapter::new());
    }
    let base = std::env::var("TOKO_API_BASE_URL")
        .unwrap_or_else(|_| "https://piutang.banza.web.id".into());
    Box::new(HttpApiAdapter::new(base))
}

pub(crate) fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            cx.bind_keys([
                gpui::KeyBinding::new("tab", crate::ModalNextField, Some("DesktopModal")),
                gpui::KeyBinding::new("shift-tab", crate::ModalPreviousField, Some("DesktopModal")),
            ]);
            let bounds = Bounds::centered(None, size(px(1240.0), px(760.0)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Keuangan Toko — Aplikasi Desktop Native".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    window.activate_window();
                    cx.new(|cx| Desktop::new(window, cx))
                },
            )
            .expect("gagal membuka jendela aplikasi desktop");
            cx.activate(true);
        });
}
