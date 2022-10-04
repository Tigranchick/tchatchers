//! Gathers all the API used to do CRUD operations on user entity.

// Copyright ⓒ 2022 LABEYE Loïc
// This tool is distributed under the MIT License, check out [here](https://github.com/nag763/tchatchers/blob/main/LICENSE.MD).

use crate::extractor::JwtUserExtractor;
use crate::State;
use crate::JWT_PATH;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use magic_crypt::MagicCryptTrait;
use std::sync::Arc;
use tchatchers_core::jwt::Jwt;
use tchatchers_core::user::{AuthenticableUser, InsertableUser, UpdatableUser, User};
use tokio::time::{sleep, Duration};
use tower_cookies::{Cookie, Cookies};

/// Creates a user.
///
/// The password will be encrypted server side.
///
/// # Arguments
///
/// - new_user : The user to insert in database.
/// - state : The data shared across thread.
pub async fn create_user(
    Json(mut new_user): Json<InsertableUser>,
    Extension(state): Extension<Arc<State>>,
) -> impl IntoResponse {
    if User::login_exists(&new_user.login, &state.pg_pool).await {
        return (
            StatusCode::BAD_REQUEST,
            "A user with a similar login already exists",
        );
    }

    new_user.password = state.encrypter.encrypt_str_to_base64(&new_user.password);

    match new_user.insert(&state.pg_pool).await {
        Ok(_) => (StatusCode::CREATED, "User created with success"),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "An error happened"),
    }
}

/// Check whether a login exists or not.
///
/// Useful when it is needed to create a new user for instance.
///
/// # Arguments
///
/// - login : The login to check.
/// - state : The data shared across thread.
pub async fn login_exists(
    Path(login): Path<String>,
    Extension(state): Extension<Arc<State>>,
) -> impl IntoResponse {
    let response_status: StatusCode = match User::login_exists(&login, &state.pg_pool).await {
        false => StatusCode::OK,
        true => StatusCode::CONFLICT,
    };
    (response_status, ())
}

/// Authenticate a user.
///
/// If the call to the service is successful, an authentication cookie will be
/// added to the user's browser.
///
/// # Arguments
/// - user : The user to authenticate.
/// - state : The data shared across thread.
/// - cookies : The user's cookies.
pub async fn authenticate(
    Json(mut user): Json<AuthenticableUser>,
    Extension(state): Extension<Arc<State>>,
    cookies: Cookies,
) -> impl IntoResponse {
    user.password = state.encrypter.encrypt_str_to_base64(&user.password);
    let user = match user.authenticate(&state.pg_pool).await {
        Some(v) => v,
        None => {
            sleep(Duration::from_secs(3)).await;
            return (StatusCode::NOT_FOUND, "We couldn't connect you, please ensure that the login and password are correct before trying again");
        }
    };
    match user.is_authorized {
        true => {
            let jwt = Jwt::from(user);
            let serialized_jwt : String = jwt.serialize(&state.jwt_secret).unwrap();
            let mut jwt_cookie = Cookie::new(JWT_PATH, serialized_jwt);
            jwt_cookie.set_path("/");
            jwt_cookie.make_permanent();
            jwt_cookie.set_secure(true);
            jwt_cookie.set_http_only(false);
            cookies.add(jwt_cookie);
            (StatusCode::OK, "")
        }
        false => (StatusCode::UNAUTHORIZED, "This user's access has been revoked, contact an admin if you believe you should access this service")
    }
}

/// Log the user out.
///
/// This will erase the cookie from the user's browser.
///
/// # Arguments
///
/// - cookies : The user's cookies.
pub async fn logout(cookies: Cookies) -> impl IntoResponse {
    let mut jwt_cookie = Cookie::new(JWT_PATH, "");
    jwt_cookie.set_path("/");
    jwt_cookie.make_removal();
    cookies.add(jwt_cookie);
    (StatusCode::OK, "")
}

/// Checks whether the authentication is legit, or if the user is authenticated
/// or not.
///
/// # Arguments
///
/// - jwt : The user's authentication token.
pub async fn validate(jwt: Option<JwtUserExtractor>) -> impl IntoResponse {
    match jwt {
        Some(_) => (StatusCode::OK, ""),
        None => (StatusCode::UNAUTHORIZED, "You aren't logged in."),
    }
}

/// Update the user's informations.
///
/// There is a check server side to ensure that the user is only able to update
/// himself.
///
/// # Arguments
/// - jwt : The user authentication token.
/// - user : the new informations to update the user.
/// - state : The data shared across thread.
/// - cookies : The user's cookies.
pub async fn update_user(
    JwtUserExtractor(jwt): JwtUserExtractor,
    Json(user): Json<UpdatableUser>,
    Extension(state): Extension<Arc<State>>,
    cookies: Cookies,
) -> impl IntoResponse {
    if jwt.user.id == user.id {
        match user.update(&state.pg_pool).await {
            Ok(_) => {
                let updated_user = User::find_by_id(user.id, &state.pg_pool).await.unwrap();
                let jwt = Jwt::from(updated_user);
                let serialized_jwt: String = jwt.serialize(&state.jwt_secret).unwrap();
                let mut jwt_cookie = Cookie::new(JWT_PATH, serialized_jwt);
                jwt_cookie.set_path("/");
                jwt_cookie.make_permanent();
                jwt_cookie.set_secure(true);
                jwt_cookie.set_http_only(false);
                cookies.add(jwt_cookie);
                (StatusCode::CREATED, "User updated with success").into_response()
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "An error happened").into_response(),
        }
    } else {
        (StatusCode::FORBIDDEN, "You can't update another user").into_response()
    }
}
