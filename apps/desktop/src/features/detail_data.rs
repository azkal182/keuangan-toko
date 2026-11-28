use crate::app::navigation::CustomerTab;
use crate::store::{CustomerDetail, LedgerPage, PaymentRow};
use chrono::NaiveDate;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DetailKey {
    pub customer_id: String,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub order_desc: bool,
    pub tab: CustomerTab,
}

#[derive(Clone)]
pub(crate) struct DetailData {
    pub customer: CustomerDetail,
    pub ledger: LedgerPage,
    pub payments: Vec<PaymentRow>,
}
