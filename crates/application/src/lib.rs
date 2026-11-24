pub use toko_domain::identity::Role;

#[derive(Debug, Clone, Copy)]
pub enum Permission {
    Authenticated,
    CustomersRead,
    CustomersWrite,
    CustomersMerge,
    LedgerRead,
    PaymentsRead,
    PaymentsWrite,
    CreditsApply,
    SalesRead,
    SalesWrite,
    ReportsRead,
    ImportsUpload,
    ImportsRead,
    ImportsReview,
    ImportsOverride,
    ImportsCommit,
    TransactionsReverse,
    CreditsReallocate,
}

impl Permission {
    pub fn allows(self, role: Role) -> bool {
        match self {
            Self::Authenticated
            | Self::CustomersRead
            | Self::LedgerRead
            | Self::PaymentsRead
            | Self::SalesRead
            | Self::ReportsRead => true,
            Self::ImportsUpload
            | Self::CustomersWrite
            | Self::SalesWrite
            | Self::PaymentsWrite
            | Self::CreditsApply => {
                matches!(role, Role::Owner | Role::Operator)
            }
            Self::CustomersMerge
            | Self::ImportsRead
            | Self::ImportsReview
            | Self::ImportsOverride
            | Self::ImportsCommit
            | Self::TransactionsReverse
            | Self::CreditsReallocate => role == Role::Owner,
        }
    }
}
