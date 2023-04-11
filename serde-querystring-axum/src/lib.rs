#![doc = include_str!("../README.md")]

use async_trait::async_trait;
use axum_core::{
    extract::FromRequestParts,
    response::{IntoResponse, Response},
};
use http::{request::Parts, StatusCode};
use serde::de::DeserializeOwned;
use serde_querystring::de::Error;

pub use serde_querystring::de::ParseMode;

pub trait QueryStringMode {
    fn get_mode() -> ParseMode {
        ParseMode::UrlEncoded
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QueryString<T: QueryStringMode>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for QueryString<T>
where
    T: DeserializeOwned + QueryStringMode,
    S: Send + Sync,
{
    type Rejection = QueryStringRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let value =
            serde_querystring::from_str(query, T::get_mode()).map_err(QueryStringRejection)?;
        Ok(QueryString(value))
    }
}

#[derive(Debug)]
pub struct QueryStringRejection(pub Error);

impl IntoResponse for QueryStringRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to deserialize query string: {}", self.0),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use axum::{
        body::{Body, HttpBody},
        extract::FromRequest,
        routing::get,
        Router,
    };
    use http::{Request, StatusCode};
    use serde::Deserialize;
    use tower::ServiceExt;

    use super::*;

    async fn check<T>(uri: impl AsRef<str>, value: T)
    where
        T: DeserializeOwned + PartialEq + Debug + QueryStringMode,
    {
        let req = Request::builder().uri(uri.as_ref()).body(()).unwrap();
        assert_eq!(
            QueryString::<T>::from_request(req, &()).await.unwrap().0,
            value
        );
    }

    #[tokio::test]
    async fn test_query() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Pagination {
            size: Option<u64>,
            pages: Option<Vec<u64>>,
        }

        impl QueryStringMode for Pagination {}

        check(
            "http://example.com/test",
            Pagination {
                size: None,
                pages: None,
            },
        )
        .await;

        check(
            "http://example.com/test?size=10",
            Pagination {
                size: Some(10),
                pages: None,
            },
        )
        .await;

        check(
            "http://example.com/test?size=10&pages=20",
            Pagination {
                size: Some(10),
                pages: Some(vec![20]),
            },
        )
        .await;

        check(
            "http://example.com/test?size=10&pages=20&pages=21&pages=22",
            Pagination {
                size: Some(10),
                pages: Some(vec![20, 21, 22]),
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_config_mode() {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Params {
            n: Vec<i32>,
        }

        impl QueryStringMode for Params {
            fn get_mode() -> ParseMode {
                ParseMode::Brackets
            }
        }

        async fn handler(QueryString(params): QueryString<Params>) -> String {
            format!("{}-{}", params.n.get(0).unwrap(), params.n.get(2).unwrap())
        }

        let app = Router::new().route("/", get(handler));
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/?n[3]=300&n[2]=200&n[1]=100")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (parts, mut body) = res.into_parts();

        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(body.data().await.unwrap().unwrap(), "100-300")
    }

    #[tokio::test]
    async fn correct_rejection_default() {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Params {
            n: i32,
        }

        impl QueryStringMode for Params {}

        async fn handler(_: QueryString<Params>) {}

        let app = Router::new().route("/", get(handler));
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/?n=string")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (parts, mut body) = res.into_parts();

        assert_eq!(parts.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            body.data().await.unwrap().unwrap(),
            "Failed to deserialize query string"
        );
    }

    #[tokio::test]
    async fn correct_rejection_custom() {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Params {
            n: i32,
        }

        impl QueryStringMode for Params {}

        async fn handler(
            x: Result<QueryString<Params>, QueryStringRejection>,
        ) -> impl IntoResponse {
            match x {
                Ok(QueryString(_)) => (StatusCode::OK, ""),
                Err(QueryStringRejection(e)) => (StatusCode::BAD_GATEWAY, "Something went wrong"),
            }
        }

        let app = Router::new().route("/", get(handler));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/?n=string")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (parts, mut body) = res.into_parts();

        assert_eq!(parts.status, StatusCode::BAD_GATEWAY);
        assert_eq!(body.data().await.unwrap().unwrap(), "Something went wrong");
    }
}
