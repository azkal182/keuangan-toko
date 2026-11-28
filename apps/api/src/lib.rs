pub mod auth;
mod customers;
mod error;
mod imports;
mod ledger;
mod payments;
pub mod pdf;
mod reports;
mod sales;
mod transactions;
mod users;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{MethodRouter, delete, get, patch, post},
};
use sqlx::PgPool;
use toko_application::Permission;

pub fn app(pool: PgPool) -> Router {
    let protected = |route: MethodRouter<PgPool>, permission| {
        route.route_layer(middleware::from_fn_with_state(
            auth::AccessPolicy::new(pool.clone(), permission),
            auth::require_access,
        ))
    };
    Router::new()
        .route(
            "/api/v1/auth/login",
            post(auth::login).with_state(auth::LoginState::new(pool.clone())),
        )
        .route(
            "/api/v1/auth/me",
            protected(get(auth::get_me), Permission::Authenticated),
        )
        .route(
            "/api/v1/auth/logout",
            protected(post(auth::logout), Permission::Authenticated),
        )
        .route(
            "/api/v1/auth/password",
            protected(post(users::change_password), Permission::Authenticated),
        )
        .route(
            "/api/v1/users",
            protected(get(users::list), Permission::UsersManage)
                .merge(protected(post(users::create), Permission::UsersManage)),
        )
        .route(
            "/api/v1/users/{id}",
            protected(patch(users::update), Permission::UsersManage).merge(protected(
                delete(users::deactivate),
                Permission::UsersManage,
            )),
        )
        .route(
            "/api/v1/users/{id}/password-reset",
            protected(post(users::reset_password), Permission::UsersManage),
        )
        .route(
            "/api/v1/customers",
            protected(get(customers::list), Permission::CustomersRead).merge(protected(
                post(customers::create),
                Permission::CustomersWrite,
            )),
        )
        .route(
            "/api/v1/customers/{id}",
            protected(get(customers::detail), Permission::CustomersRead).merge(protected(
                patch(customers::update),
                Permission::CustomersWrite,
            )),
        )
        .route(
            "/api/v1/customers/{id}/ledger",
            protected(get(ledger::list), Permission::LedgerRead),
        )
        .route(
            "/api/v1/customers/{id}/ledger/export-pdf",
            protected(get(ledger::export_bill_pdf), Permission::LedgerRead),
        )
        .route(
            "/api/v1/customers/{id}/payments",
            protected(get(payments::customer_list), Permission::PaymentsRead),
        )
        .route(
            "/api/v1/customers/{id}/credit-applications",
            protected(post(payments::apply), Permission::CreditsApply),
        )
        .route(
            "/api/v1/payments",
            protected(post(payments::create), Permission::PaymentsWrite),
        )
        .route(
            "/api/v1/payments/{id}",
            protected(get(payments::get), Permission::PaymentsRead),
        )
        .route(
            "/api/v1/reports/receivables-aging",
            protected(get(reports::receivables_aging), Permission::ReportsRead),
        )
        .route(
            "/api/v1/reports/receivables-aging/customers",
            protected(
                get(reports::receivables_aging_customers),
                Permission::ReportsRead,
            ),
        )
        .route(
            "/api/v1/customers/{id}/merge",
            protected(post(customers::merge), Permission::CustomersMerge),
        )
        .route(
            "/api/v1/credit-sales",
            protected(post(sales::create), Permission::SalesWrite),
        )
        .route(
            "/api/v1/credit-sales/{id}",
            protected(get(sales::get), Permission::SalesRead)
                .merge(protected(patch(sales::update), Permission::SalesWrite)),
        )
        .route(
            "/api/v1/credit-sales/{id}/post",
            protected(post(sales::post), Permission::SalesWrite),
        )
        .route(
            "/api/v1/transactions/{id}/reversal",
            protected(post(transactions::reverse), Permission::TransactionsReverse),
        )
        .route(
            "/api/v1/transactions/{id}/replacement",
            protected(
                post(transactions::link_replacement),
                Permission::TransactionsReverse,
            ),
        )
        .route(
            "/api/v1/payments/{id}/reallocation",
            protected(
                post(transactions::reallocate),
                Permission::CreditsReallocate,
            ),
        )
        .route(
            "/api/v1/import-batches",
            protected(get(imports::list_batches), Permission::ImportsRead)
                .merge(protected(post(imports::upload), Permission::ImportsUpload))
                .layer(DefaultBodyLimit::max(25 * 1024 * 1024 + 64 * 1024)),
        )
        .route(
            "/api/v1/import-batches/{id}",
            protected(get(imports::get_batch), Permission::ImportsRead),
        )
        .route(
            "/api/v1/import-batches/{id}/rows",
            protected(get(imports::list_rows), Permission::ImportsRead),
        )
        .route(
            "/api/v1/import-batches/{id}/customer-identities",
            protected(
                get(imports::list_customer_identities),
                Permission::ImportsRead,
            ),
        )
        .route(
            "/api/v1/import-batches/{id}/customer-identities/prepare",
            protected(
                post(imports::prepare_customer_identities),
                Permission::ImportsReview,
            ),
        )
        .route(
            "/api/v1/import-batches/{id}/rows/{row_id}",
            protected(patch(imports::review_row), Permission::ImportsReview),
        )
        .route(
            "/api/v1/import-batches/{id}/commit",
            protected(post(imports::commit_batch), Permission::ImportsCommit),
        )
        .with_state(pool)
        .layer(middleware::from_fn(auth::request_context))
}
