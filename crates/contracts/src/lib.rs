pub mod auth;
pub mod customers;
pub mod imports;
pub mod reports;
pub mod sales;
pub mod transactions;
pub mod users;

use auth::SessionSecurity;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub error: ErrorDetails,
    pub request_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct EmptyMeta {}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PageMeta {
    #[schema(required = true)]
    pub next_cursor: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Keuangan Toko API", version = "1"),
    paths(auth::get_me, auth::login, auth::logout,
          customers::list_customers, customers::create_customer, customers::get_customer,
          customers::update_customer, customers::merge_customer, customers::list_ledger,
          sales::create_sale, sales::update_sale, sales::post_sale, sales::get_sale,
          customers::create_payment, customers::customer_payments, customers::payment_detail, customers::apply_credit,
          reports::receivables_aging, reports::receivables_aging_customers,
          transactions::reverse_transaction, transactions::link_replacement, transactions::reallocate_payment,
          imports::upload_import, imports::list_imports, imports::get_import,
          imports::list_import_rows, imports::list_import_customer_identities,
          imports::prepare_import_customer_identities,
          imports::review_import_row, imports::commit_import,
          users::list_users, users::create_user, users::update_user, users::deactivate_user,
          users::change_password, users::reset_user_password),
    modifiers(&SessionSecurity),
    components(schemas(
        ApiError, ErrorDetails, EmptyMeta, PageMeta,
        auth::CurrentUser, auth::SessionDetails, auth::SessionResponse, auth::UserRole,
        auth::LoginRequest, auth::LogoutData, auth::LogoutResponse,
        users::User, users::UserListResponse, users::UserResponse,
        users::CreateUser, users::UpdateUser, users::ResetUserPassword,
        users::ChangePassword, users::PasswordChanged, users::PasswordChangedResponse,
        customers::Customer, customers::CustomerDetail, customers::CustomerList,
        customers::ListCustomersQuery, customers::CreateCustomer, customers::UpdateCustomer,
        customers::MergeCustomer, customers::LedgerQuery, customers::LedgerEntry,
        customers::LedgerPage, customers::LedgerSummary,
        customers::CustomerListResponse, customers::CustomerResponse,
        customers::CustomerDetailResponse, customers::LedgerResponse,
        customers::CreatePayment, customers::PaymentMethod, customers::PaymentDetail,
        customers::PaymentResponse, customers::PaymentListResponse, customers::PaymentAllocation,
        customers::PaymentsQuery, customers::ApplyCredit, customers::CreditApplicationResponse,
        reports::AgingBucket, reports::AgingQuery, reports::AgingSummary, reports::AgingResponse,
        reports::AgingCustomersQuery, reports::AgingCustomer, reports::AgingCustomersResponse,
        transactions::ReverseTransaction, transactions::ReversalData, transactions::ReversalResponse,
        transactions::LinkReplacement, transactions::CorrectionLinkData, transactions::CorrectionLinkResponse,
        transactions::ReallocatePayment, transactions::ReplacementAllocation, transactions::ReallocationData, transactions::ReallocationResponse,
        imports::ImportReviewAction, imports::ImportBatchSummary, imports::ImportBatchResponse,
        imports::ImportBatchListResponse, imports::ImportRow, imports::ImportCustomerSource,
        imports::ImportCustomerIdentityPreview, imports::ImportCustomerIdentitiesResponse,
        imports::PrepareImportCustomerIdentitiesData,
        imports::PrepareImportCustomerIdentitiesResponse,
        imports::ImportRowsQuery,
        imports::ImportRowsResponse, imports::ReviewImportRow, imports::ImportReviewResponse,
        imports::CommitImport, imports::ImportCommitData, imports::ImportCommitResponse,
        imports::UploadImport
    ))
)]
pub struct ApiDoc;
