#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentView {
    CustomerList,
    CustomerDetail,
    AgingReport,
    UserManagement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomerTab {
    Ledger,
    Payments,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActiveModal {
    Login,
    ChangePassword,
    NewCustomer,
    EditCustomer {
        customer_id: String,
    },
    MergeCustomer {
        source_id: String,
    },
    RecordSale,
    RecordPayment,
    ApplyDeposit,
    ReallocatePayment {
        payment_id: String,
    },
    ReverseTransaction {
        customer_id: String,
        entry_id: String,
    },
    FilterLedgerDate,
    NewUser,
    EditUser {
        user_id: String,
    },
    ResetUserPassword {
        user_id: String,
        username: String,
    },
    ChangeAsOfDate,
    Notice {
        title: &'static str,
        message: String,
    },
}
