use crate::store::UserItem;
use gpui::{App, Context, IntoElement, ParentElement, Window, div, px};
use gpui_kit::component::{
    label::Label,
    table::{Column, TableDelegate, TableState},
};

#[derive(Default)]
pub(crate) struct AccountRows {
    pub rows: Vec<UserItem>,
    pub current_user_id: String,
    signature: String,
}

impl AccountRows {
    pub fn replace(&mut self, rows: Vec<UserItem>, current_user_id: String) -> bool {
        let signature = format!("{rows:?}|{current_user_id}");
        if self.signature == signature {
            return false;
        }
        self.rows = rows;
        self.current_user_id = current_user_id;
        self.signature = signature;
        true
    }
}

impl TableDelegate for AccountRows {
    fn columns_count(&self, _: &App) -> usize {
        5
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let (key, label, width) = [
            ("username", "Nama pengguna", 220.),
            ("role", "Peran", 180.),
            ("status", "Status akun", 150.),
            ("password", "Kewajiban sandi", 190.),
            ("created", "Tanggal dibuat", 160.),
        ][ix];
        Column::new(key, label).width(px(width)).movable(false)
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let user = &self.rows[row];
        let value = match col {
            0 if user.id == self.current_user_id => format!("{} (Akun Anda)", user.username),
            0 => user.username.clone(),
            1 => user.role.label().to_string(),
            2 => if user.is_active { "Aktif" } else { "Nonaktif" }.into(),
            3 => if user.must_change_password {
                "Wajib ganti sandi"
            } else {
                "Terverifikasi"
            }
            .into(),
            _ => user.created_at.format("%d %b %Y").to_string(),
        };
        div().child(Label::new(value))
    }
}
