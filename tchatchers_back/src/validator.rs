// Copyright ⓒ 2022 LABEYE Loïc
// This tool is distributed under the MIT License, check out [here](https://github.com/nag763/tchatchers/blob/main/LICENSE.MD).

//! Extractor used mainly to verify that the incoming fields are valid before processing the requests.
//!
//! So far, only the JSON inputs are implemented, but others wouldn't be much harder to implement.

use axum::{
    async_trait,
    body::HttpBody,
    extract::{rejection::JsonRejection, FromRequest},
    http::Request,
    response::IntoResponse,
    BoxError, Json as JsonAxum,
};
use serde::de::DeserializeOwned;
use tchatchers_core::validation_error_message::ValidationErrorMessage;
use validator::{Validate, ValidationErrors};

use crate::AppState;

/// Errors returned on validation.
pub enum JsonValidatorRejection {
    /// Axum's own validation errors.
    JsonAxumRejection(JsonRejection),
    /// The one returned from the validator.
    ValidationRejection(ValidationErrors),
}

impl IntoResponse for JsonValidatorRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            JsonValidatorRejection::JsonAxumRejection(rej) => rej.into_response(),
            JsonValidatorRejection::ValidationRejection(errors) => {
                let validation_error_message: ValidationErrorMessage =
                    ValidationErrorMessage::from(errors);
                validation_error_message.into_response()
            }
        }
    }
}

/// A validated JSON input.
///
/// Mainly used to validate the data before processing it server side.
pub struct ValidJson<T>(pub T)
where
    T: Validate;

#[async_trait]
impl<B, T> FromRequest<AppState, B> for ValidJson<T>
where
    B: 'static + Send + HttpBody,
    B::Data: Send,
    B::Error: Into<BoxError>,
    T: Validate + Sized + DeserializeOwned,
{
    type Rejection = JsonValidatorRejection;

    async fn from_request(req: Request<B>, state: &AppState) -> Result<Self, Self::Rejection> {
        match JsonAxum::<T>::from_request(req, state).await {
            Ok(json_value) => {
                if let Err(e) = json_value.validate() {
                    return Err(JsonValidatorRejection::ValidationRejection(e));
                }
                return Ok(ValidJson(json_value.0));
            }
            Err(e) => return Err(JsonValidatorRejection::JsonAxumRejection(e)),
        };
    }
}
