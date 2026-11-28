use gpui_kit::component::label::Label;
use gpui_kit::component::table::{Column, ColumnSort, TableDelegate, TableState};
use gpui_kit::{App, Context, IntoElement, ParentElement, Window, div, px};

#[derive(Clone)]
struct Customer {
    name: String,
    balance: u32,
}
pub struct Customers {
    rows: Vec<Customer>,
    query: String,
}
impl Customers {
    pub fn new() -> Self {
        Self {
            rows: (1..=120)
                .map(|n| Customer {
                    name: format!("Pelanggan Contoh {n:03}"),
                    balance: (n % 7) * 25000,
                })
                .collect(),
            query: String::new(),
        }
    }
    pub fn filter(&mut self, query: &str) {
        self.query = query.to_lowercase();
    }
    fn sort(&mut self, col: usize, sort: ColumnSort) {
        self.rows.sort_by(|a, b| match col {
            0 => a.name.cmp(&b.name),
            1 => (a.balance > 0).cmp(&(b.balance > 0)),
            _ => a.balance.cmp(&b.balance),
        });
        if sort == ColumnSort::Descending {
            self.rows.reverse();
        }
    }
    fn visible(&self) -> Vec<&Customer> {
        self.rows
            .iter()
            .filter(|row| row.name.to_lowercase().contains(&self.query))
            .collect()
    }
}
impl TableDelegate for Customers {
    fn columns_count(&self, _: &App) -> usize {
        3
    }
    fn rows_count(&self, _: &App) -> usize {
        self.visible().len()
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let (key, name, width) = [
            ("name", "Pelanggan", 280.),
            ("status", "Status", 160.),
            ("balance", "Piutang (Rp)", 160.),
        ][ix];
        // Column identity is positional in this experiment; do not allow reordering.
        let col = Column::new(key, name)
            .width(px(width))
            .sortable()
            .movable(false);
        if ix == 2 { col.text_right() } else { col }
    }
    fn perform_sort(
        &mut self,
        col: usize,
        sort: ColumnSort,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        self.sort(col, sort);
        cx.notify();
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let rows = self.visible();
        let item = rows[row];
        let text = match col {
            0 => item.name.clone(),
            1 => if item.balance == 0 {
                "Lunas"
            } else {
                "Belum lunas"
            }
            .to_string(),
            _ => item.balance.to_string(),
        };
        div().child(Label::new(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtering_and_numeric_sort_keep_synthetic_rows_consistent() {
        let mut customers = Customers::new();
        customers.filter("contoh 00");
        assert_eq!(customers.visible().len(), 9);
        customers.sort(2, ColumnSort::Ascending);
        let amounts: Vec<_> = customers.visible().iter().map(|row| row.balance).collect();
        assert!(amounts.windows(2).all(|pair| pair[0] <= pair[1]));
        customers.sort(2, ColumnSort::Descending);
        let amounts: Vec<_> = customers.visible().iter().map(|row| row.balance).collect();
        assert!(amounts.windows(2).all(|pair| pair[0] >= pair[1]));
        customers.filter("tidak ada");
        assert!(customers.visible().is_empty());
    }
}
