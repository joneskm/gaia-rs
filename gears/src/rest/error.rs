use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::baseapp::errors::QueryError;

#[derive(Debug)]
pub struct HTTPError {
    status: StatusCode,
    description: String, // set to (unless overridden): https://github.com/SergioBenitez/Rocket/blob/91f6288ea4aeb3d5a502b2f18b2b9677a85463ea/core/lib/src/catcher/catcher.rs#L369-L416
}

#[derive(Serialize)]
struct PrintErrorCore {
    pub code: u16,
    pub reason: String,
    pub description: String,
}

#[derive(Serialize)]
struct PrintError {
    error: PrintErrorCore,
}

impl HTTPError {
    pub fn bad_request(description: String) -> HTTPError {
        HTTPError {
            status: StatusCode::BAD_REQUEST,
            description,
        }
    }

    pub fn not_found() -> HTTPError {
        HTTPError {
            status: StatusCode::NOT_FOUND,
            description: "The requested resource could not be found.".into(),
        }
    }

    pub fn not_found_with_msg(description: String) -> HTTPError {
        HTTPError {
            status: StatusCode::NOT_FOUND,
            description,
        }
    }

    pub fn internal_server_error() -> HTTPError {
        HTTPError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            description: "An internal server error occurred.".into(),
        }
    }

    pub fn gateway_timeout() -> HTTPError {
        HTTPError {
            status: StatusCode::GATEWAY_TIMEOUT,
            description: "The server did not receive a timely response from an upstream server."
                .into(),
        }
    }

    pub fn bad_gateway() -> HTTPError {
        HTTPError {
            status: StatusCode::BAD_GATEWAY,
            description: "The server received an invalid response from an upstream server".into(),
        }
    }

    pub fn bad_gateway_with_msg(description: String) -> HTTPError {
        HTTPError {
            status: StatusCode::BAD_GATEWAY,
            description,
        }
    }

    fn into_serializable(self) -> PrintError {
        PrintError {
            error: PrintErrorCore {
                code: self.status.as_u16(),
                reason: self
                    .status
                    .canonical_reason()
                    .unwrap_or_default()
                    .to_string(),
                description: self.description,
            },
        }
    }
}

impl IntoResponse for HTTPError {
    fn into_response(self) -> Response {
        (self.status, Json(self.into_serializable())).into_response()
    }
}

impl From<QueryError> for HTTPError {
    fn from(err: QueryError) -> Self {
        match err {
            QueryError::Store(_) => {
                HTTPError::not_found_with_msg("The requested version could not be found.".into())
            }
            _ => HTTPError::bad_request("Invalid request.".to_owned()), // TODO: Don't forget to add more info later
        }
    }
}
