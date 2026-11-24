use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Path, Query, Request},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use toko_contracts::{ApiError, ErrorDetails};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct RequestId(pub Uuid);

impl<S: Sync> FromRequestParts<S> for RequestId {
    type Rejection = Failure;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Failure> {
        parts
            .extensions
            .get::<Self>()
            .copied()
            .ok_or_else(|| Self(Uuid::new_v4()).internal())
    }
}

pub struct Failure(StatusCode, ApiError);

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}

impl RequestId {
    pub fn error(self, status: StatusCode, code: &str, message: &str) -> Failure {
        Failure(
            status,
            ApiError {
                error: ErrorDetails {
                    code: code.into(),
                    message: message.into(),
                    fields: Default::default(),
                },
                request_id: self.0,
            },
        )
    }

    pub fn field(self, status: StatusCode, code: &str, field: &str, message: &str) -> Failure {
        let mut failure = self.error(status, code, message);
        failure.1.error.fields.insert(field.into(), message.into());
        failure
    }

    pub fn invalid(self, field: &str, message: &str) -> Failure {
        self.field(
            StatusCode::UNPROCESSABLE_ENTITY,
            "VALIDATION_FAILED",
            field,
            message,
        )
    }

    pub fn internal(self) -> Failure {
        eprintln!("level=error event=request_failed request_id={}", self.0);
        self.error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Terjadi kesalahan internal",
        )
    }

    pub fn missing_customer(self) -> Failure {
        self.error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Pelanggan tidak ditemukan",
        )
    }
}

pub struct Body<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequest<S> for Body<T> {
    type Rejection = Failure;

    async fn from_request(request: Request, state: &S) -> Result<Self, Failure> {
        let id = *request
            .extensions()
            .get::<RequestId>()
            .ok_or_else(|| RequestId(Uuid::new_v4()).internal())?;
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|error| {
                let status = error.status();
                id.error(
                    if status == StatusCode::PAYLOAD_TOO_LARGE {
                        status
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "INVALID_BODY",
                    "Isi permintaan tidak valid",
                )
            })
    }
}

pub struct Params<T>(pub T);
pub struct RouteId(pub Uuid);

impl<S: Send + Sync, T: DeserializeOwned> FromRequestParts<S> for Params<T> {
    type Rejection = Failure;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Failure> {
        let id = RequestId::from_request_parts(parts, state).await?;
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(|_| {
                id.error(
                    StatusCode::BAD_REQUEST,
                    "INVALID_QUERY",
                    "Parameter kueri tidak valid",
                )
            })
    }
}

impl<S: Send + Sync> FromRequestParts<S> for RouteId {
    type Rejection = Failure;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Failure> {
        let id = RequestId::from_request_parts(parts, state).await?;
        Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map(|Path(value)| Self(value))
            .map_err(|_| {
                id.error(
                    StatusCode::BAD_REQUEST,
                    "INVALID_ID",
                    "ID pelanggan tidak valid",
                )
            })
    }
}
