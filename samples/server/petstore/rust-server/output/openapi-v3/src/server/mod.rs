use std::marker::PhantomData;
use futures::{Future, future, Stream, stream};
use hyper;
use hyper::{Request, Response, Error, StatusCode, Body, HeaderMap};
use hyper::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use url::form_urlencoded;
use serde_json;
use std::io;
#[allow(unused_imports)]
use swagger;
use swagger::{ApiError, XSpanIdString, Has, RequestParser};
pub use swagger::auth::Authorization;
use swagger::auth::Scopes;
use swagger::context::ContextualPayload;
use uuid;
use serde_xml_rs;

#[allow(unused_imports)]
use models;
use header;

pub use crate::context;

use {Api,
     CallbackWithHeaderPostResponse,
     ComplexQueryParamGetResponse,
     EnumInPathPathParamGetResponse,
     MandatoryRequestHeaderGetResponse,
     MergePatchJsonGetResponse,
     MultigetGetResponse,
     MultipleAuthSchemeGetResponse,
     OverrideServerGetResponse,
     ParamgetGetResponse,
     ReadonlyAuthSchemeGetResponse,
     RegisterCallbackPostResponse,
     RequiredOctetStreamPutResponse,
     ResponsesWithHeadersGetResponse,
     Rfc7807GetResponse,
     UntypedPropertyGetResponse,
     UuidGetResponse,
     XmlExtraPostResponse,
     XmlOtherPostResponse,
     XmlOtherPutResponse,
     XmlPostResponse,
     XmlPutResponse,
     CreateRepoResponse,
     Get RepoInfoResponse
};

pub mod callbacks;

mod paths {
    extern crate regex;

    lazy_static! {
        pub static ref GLOBAL_REGEX_SET: regex::RegexSet = regex::RegexSet::new(vec![
            r"^/callback-with-header$",
            r"^/complex-query-param$",
            r"^/enum_in_path/(?P<path_param>[^/?#]*)$",
            r"^/mandatory-request-header$",
            r"^/merge-patch-json$",
            r"^/multiget$",
            r"^/multiple_auth_scheme$",
            r"^/override-server$",
            r"^/paramget$",
            r"^/readonly_auth_scheme$",
            r"^/register-callback$",
            r"^/repos$",
            r"^/repos/(?P<repoId>[^/?#]*)$",
            r"^/required_octet_stream$",
            r"^/responses_with_headers$",
            r"^/rfc7807$",
            r"^/untyped_property$",
            r"^/uuid$",
            r"^/xml$",
            r"^/xml_extra$",
            r"^/xml_other$"
        ])
        .expect("Unable to create global regex set");
    }
    pub static ID_CALLBACK_WITH_HEADER: usize = 0;
    pub static ID_COMPLEX_QUERY_PARAM: usize = 1;
    pub static ID_ENUM_IN_PATH_PATH_PARAM: usize = 2;
    lazy_static! {
        pub static ref REGEX_ENUM_IN_PATH_PATH_PARAM: regex::Regex =
            regex::Regex::new(r"^/enum_in_path/(?P<path_param>[^/?#]*)$")
                .expect("Unable to create regex for ENUM_IN_PATH_PATH_PARAM");
    }
    pub static ID_MANDATORY_REQUEST_HEADER: usize = 3;
    pub static ID_MERGE_PATCH_JSON: usize = 4;
    pub static ID_MULTIGET: usize = 5;
    pub static ID_MULTIPLE_AUTH_SCHEME: usize = 6;
    pub static ID_OVERRIDE_SERVER: usize = 7;
    pub static ID_PARAMGET: usize = 8;
    pub static ID_READONLY_AUTH_SCHEME: usize = 9;
    pub static ID_REGISTER_CALLBACK: usize = 10;
    pub static ID_REPOS: usize = 11;
    pub static ID_REPOS_REPOID: usize = 12;
    lazy_static! {
        pub static ref REGEX_REPOS_REPOID: regex::Regex =
            regex::Regex::new(r"^/repos/(?P<repoId>[^/?#]*)$")
                .expect("Unable to create regex for REPOS_REPOID");
    }
    pub static ID_REQUIRED_OCTET_STREAM: usize = 13;
    pub static ID_RESPONSES_WITH_HEADERS: usize = 14;
    pub static ID_RFC7807: usize = 15;
    pub static ID_UNTYPED_PROPERTY: usize = 16;
    pub static ID_UUID: usize = 17;
    pub static ID_XML: usize = 18;
    pub static ID_XML_EXTRA: usize = 19;
    pub static ID_XML_OTHER: usize = 20;
}

pub struct MakeService<T, RC> {
    api_impl: T,
    marker: PhantomData<RC>,
}

impl<T, RC> MakeService<T, RC>
where
    T: Api<RC> + Clone + Send + 'static,
    RC: Has<XSpanIdString> + Has<Option<Authorization>> + 'static
{
    pub fn new(api_impl: T) -> Self {
        MakeService {
            api_impl,
            marker: PhantomData
        }
    }
}

impl<'a, T, SC, RC> hyper::service::MakeService<&'a SC> for MakeService<T, RC>
where
    T: Api<RC> + Clone + Send + 'static,
    RC: Has<XSpanIdString> + Has<Option<Authorization>> + 'static + Send
{
    type ReqBody = ContextualPayload<Body, RC>;
    type ResBody = Body;
    type Error = Error;
    type Service = Service<T, RC>;
    type Future = future::FutureResult<Self::Service, Self::MakeError>;
    type MakeError = Error;

    fn make_service(&mut self, _ctx: &'a SC) -> Self::Future {
        future::FutureResult::from(Ok(Service::new(
            self.api_impl.clone(),
        )))
    }
}

type ServiceFuture = Box<dyn Future<Item = Response<Body>, Error = Error> + Send>;

fn method_not_allowed() -> ServiceFuture {
    Box::new(future::ok(
        Response::builder().status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .expect("Unable to create Method Not Allowed response")
    ))
}

pub struct Service<T, RC> {
    api_impl: T,
    marker: PhantomData<RC>,
}

impl<T, RC> Service<T, RC>
where
    T: Api<RC> + Clone + Send + 'static,
    RC: Has<XSpanIdString> + Has<Option<Authorization>> + 'static {
    pub fn new(api_impl: T) -> Self {
        Service {
            api_impl: api_impl,
            marker: PhantomData
        }
    }
}

impl<T, C> hyper::service::Service for Service<T, C>
where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString> + Has<Option<Authorization>> + 'static + Send
{
    type ReqBody = ContextualPayload<Body, C>;
    type ResBody = Body;
    type Error = Error;
    type Future = ServiceFuture;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        let api_impl = self.api_impl.clone();
        let (parts, body) = req.into_parts();
        let (method, uri, headers) = (parts.method, parts.uri, parts.headers);
        let path = paths::GLOBAL_REGEX_SET.matches(uri.path());
        let mut context = body.context;
        let body = body.inner;

        match &method {

            // CallbackWithHeaderPost - POST /callback-with-header
            &hyper::Method::POST if path.matched(paths::ID_CALLBACK_WITH_HEADER) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_url = query_params.iter().filter(|e| e.0 == "url").map(|e| e.1.to_owned())
                    .nth(0);
                let param_url = match param_url {
                    Some(param_url) => match param_url.parse::<String>() {
                        Ok(param_url) => param_url,
                        Err(e) => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse query parameter url - doesn't match schema: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid query parameter url"))),
                    },
                    None => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from("Missing required query parameter url"))
                                        .expect("Unable to create Bad Request response for missing qeury parameter url"))),
                };

                Box::new({
                        {{
                                Box::new(
                                    api_impl.callback_with_header_post(
                                            param_url,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                CallbackWithHeaderPostResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(204).expect("Unable to turn 204 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // ComplexQueryParamGet - GET /complex-query-param
            &hyper::Method::GET if path.matched(paths::ID_COMPLEX_QUERY_PARAM) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_list_of_strings = query_params.iter().filter(|e| e.0 == "list-of-strings").map(|e| e.1.to_owned())
                    .filter_map(|param_list_of_strings| param_list_of_strings.parse().ok())
                    .collect::<Vec<_>>();
                let param_list_of_strings = if !param_list_of_strings.is_empty() {
                    Some(param_list_of_strings)
                } else {
                    None
                };

                Box::new({
                        {{
                                Box::new(
                                    api_impl.complex_query_param_get(
                                            param_list_of_strings.as_ref(),
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                ComplexQueryParamGetResponse::Success
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // EnumInPathPathParamGet - GET /enum_in_path/{path_param}
            &hyper::Method::GET if path.matched(paths::ID_ENUM_IN_PATH_PATH_PARAM) => {
                // Path parameters
                let path: &str = &uri.path().to_string();
                let path_params =
                    paths::REGEX_ENUM_IN_PATH_PATH_PARAM
                    .captures(&path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE ENUM_IN_PATH_PATH_PARAM in set but failed match against \"{}\"", path, paths::REGEX_ENUM_IN_PATH_PATH_PARAM.as_str())
                    );

                let param_path_param = match percent_encoding::percent_decode(path_params["path_param"].as_bytes()).decode_utf8() {
                    Ok(param_path_param) => match param_path_param.parse::<models::StringEnum>() {
                        Ok(param_path_param) => param_path_param,
                        Err(e) => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter path_param: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter"))),
                    },
                    Err(_) => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["path_param"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode")))
                };

                Box::new({
                        {{
                                Box::new(
                                    api_impl.enum_in_path_path_param_get(
                                            param_path_param,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                EnumInPathPathParamGetResponse::Success
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // MandatoryRequestHeaderGet - GET /mandatory-request-header
            &hyper::Method::GET if path.matched(paths::ID_MANDATORY_REQUEST_HEADER) => {
                // Header parameters
                let param_x_header = headers.get(HeaderName::from_static("x-header"));

                let param_x_header = match param_x_header {
                    Some(v) => header::IntoHeaderValue::<String>::from((*v).clone()).0,
                    None => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from("Missing or invalid required header X-Header"))
                                        .expect("Unable to create Bad Request response for missing required header X-Header"))),
                };

                Box::new({
                        {{
                                Box::new(
                                    api_impl.mandatory_request_header_get(
                                            param_x_header,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                MandatoryRequestHeaderGetResponse::Success
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // MergePatchJsonGet - GET /merge-patch-json
            &hyper::Method::GET if path.matched(paths::ID_MERGE_PATCH_JSON) => {
                Box::new({
                        {{
                                Box::new(
                                    api_impl.merge_patch_json_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                MergePatchJsonGetResponse::Merge
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/merge-patch+json")
                                                            .expect("Unable to create Content-Type header for MERGE_PATCH_JSON_GET_MERGE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // MultigetGet - GET /multiget
            &hyper::Method::GET if path.matched(paths::ID_MULTIGET) => {
                Box::new({
                        {{
                                Box::new(
                                    api_impl.multiget_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                MultigetGetResponse::JSONRsp
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for MULTIGET_GET_JSON_RSP"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                MultigetGetResponse::XMLRsp
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/xml")
                                                            .expect("Unable to create Content-Type header for MULTIGET_GET_XML_RSP"));
                                                    let body = serde_xml_rs::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                MultigetGetResponse::OctetRsp
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(202).expect("Unable to turn 202 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/octet-stream")
                                                            .expect("Unable to create Content-Type header for MULTIGET_GET_OCTET_RSP"));
                                                    let body = body.0;
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                MultigetGetResponse::StringRsp
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(203).expect("Unable to turn 203 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("text/plain")
                                                            .expect("Unable to create Content-Type header for MULTIGET_GET_STRING_RSP"));
                                                    let body = body;
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                MultigetGetResponse::DuplicateResponseLongText
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(204).expect("Unable to turn 204 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for MULTIGET_GET_DUPLICATE_RESPONSE_LONG_TEXT"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                MultigetGetResponse::DuplicateResponseLongText_2
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(205).expect("Unable to turn 205 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for MULTIGET_GET_DUPLICATE_RESPONSE_LONG_TEXT_2"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                MultigetGetResponse::DuplicateResponseLongText_3
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(206).expect("Unable to turn 206 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for MULTIGET_GET_DUPLICATE_RESPONSE_LONG_TEXT_3"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // MultipleAuthSchemeGet - GET /multiple_auth_scheme
            &hyper::Method::GET if path.matched(paths::ID_MULTIPLE_AUTH_SCHEME) => {
                {
                    let authorization = match (&context as &dyn Has<Option<Authorization>>).get() {
                        &Some(ref authorization) => authorization,
                        &None => return Box::new(future::ok(Response::builder()
                                                .status(StatusCode::FORBIDDEN)
                                                .body(Body::from("Unauthenticated"))
                                                .expect("Unable to create Authentication Forbidden response"))),
                    };

                    // Authorization
                    if let Scopes::Some(ref scopes) = authorization.scopes {
                        let required_scopes: std::collections::BTreeSet<String> = vec![
                            "test.read".to_string(), // Allowed to read state.
                            "test.write".to_string(), // Allowed to change state.
                        ].into_iter().collect();

                        if !required_scopes.is_subset(scopes) {
                            let missing_scopes = required_scopes.difference(scopes);
                            return Box::new(future::ok(Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .body(Body::from(missing_scopes.fold(
                                    "Insufficient authorization, missing scopes".to_string(),
                                    |s, scope| format!("{} {}", s, scope))
                                ))
                                .expect("Unable to create Authentication Insufficient response")
                            ));
                        }
                    }
                }

                Box::new({
                        {{
                                Box::new(
                                    api_impl.multiple_auth_scheme_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                MultipleAuthSchemeGetResponse::CheckThatLimitingToMultipleRequiredAuthSchemesWorks
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // OverrideServerGet - GET /override-server
            &hyper::Method::GET if path.matched(paths::ID_OVERRIDE_SERVER) => {
                Box::new({
                        {{
                                Box::new(
                                    api_impl.override_server_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                OverrideServerGetResponse::Success
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(204).expect("Unable to turn 204 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // ParamgetGet - GET /paramget
            &hyper::Method::GET if path.matched(paths::ID_PARAMGET) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_uuid = query_params.iter().filter(|e| e.0 == "uuid").map(|e| e.1.to_owned())
                    .nth(0);
                let param_uuid = param_uuid.and_then(|param_uuid| param_uuid.parse::<>().ok());
                let param_some_object = query_params.iter().filter(|e| e.0 == "someObject").map(|e| e.1.to_owned())
                    .nth(0);
                let param_some_object = param_some_object.and_then(|param_some_object| param_some_object.parse::<>().ok());
                let param_some_list = query_params.iter().filter(|e| e.0 == "someList").map(|e| e.1.to_owned())
                    .nth(0);
                let param_some_list = param_some_list.and_then(|param_some_list| param_some_list.parse::<>().ok());

                Box::new({
                        {{
                                Box::new(
                                    api_impl.paramget_get(
                                            param_uuid,
                                            param_some_object,
                                            param_some_list,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                ParamgetGetResponse::JSONRsp
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for PARAMGET_GET_JSON_RSP"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // ReadonlyAuthSchemeGet - GET /readonly_auth_scheme
            &hyper::Method::GET if path.matched(paths::ID_READONLY_AUTH_SCHEME) => {
                {
                    let authorization = match (&context as &dyn Has<Option<Authorization>>).get() {
                        &Some(ref authorization) => authorization,
                        &None => return Box::new(future::ok(Response::builder()
                                                .status(StatusCode::FORBIDDEN)
                                                .body(Body::from("Unauthenticated"))
                                                .expect("Unable to create Authentication Forbidden response"))),
                    };

                    // Authorization
                    if let Scopes::Some(ref scopes) = authorization.scopes {
                        let required_scopes: std::collections::BTreeSet<String> = vec![
                            "test.read".to_string(), // Allowed to read state.
                        ].into_iter().collect();

                        if !required_scopes.is_subset(scopes) {
                            let missing_scopes = required_scopes.difference(scopes);
                            return Box::new(future::ok(Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .body(Body::from(missing_scopes.fold(
                                    "Insufficient authorization, missing scopes".to_string(),
                                    |s, scope| format!("{} {}", s, scope))
                                ))
                                .expect("Unable to create Authentication Insufficient response")
                            ));
                        }
                    }
                }

                Box::new({
                        {{
                                Box::new(
                                    api_impl.readonly_auth_scheme_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                ReadonlyAuthSchemeGetResponse::CheckThatLimitingToASingleRequiredAuthSchemeWorks
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // RegisterCallbackPost - POST /register-callback
            &hyper::Method::POST if path.matched(paths::ID_REGISTER_CALLBACK) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_url = query_params.iter().filter(|e| e.0 == "url").map(|e| e.1.to_owned())
                    .nth(0);
                let param_url = match param_url {
                    Some(param_url) => match param_url.parse::<String>() {
                        Ok(param_url) => param_url,
                        Err(e) => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse query parameter url - doesn't match schema: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid query parameter url"))),
                    },
                    None => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from("Missing required query parameter url"))
                                        .expect("Unable to create Bad Request response for missing qeury parameter url"))),
                };

                Box::new({
                        {{
                                Box::new(
                                    api_impl.register_callback_post(
                                            param_url,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                RegisterCallbackPostResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(204).expect("Unable to turn 204 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // RequiredOctetStreamPut - PUT /required_octet_stream
            &hyper::Method::PUT if path.matched(paths::ID_REQUIRED_OCTET_STREAM) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let param_body: Option<swagger::ByteArray> = if !body.is_empty() {
                                    Some(swagger::ByteArray(body.to_vec()))
                                } else {
                                    None
                                };
                                let param_body = match param_body {
                                    Some(param_body) => param_body,
                                    None => return Box::new(future::ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from("Missing required body parameter body"))
                                                        .expect("Unable to create Bad Request response for missing body parameter body"))),
                                };

                                Box::new(
                                    api_impl.required_octet_stream_put(
                                            param_body,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                RequiredOctetStreamPutResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter body: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter body"))),
                        }
                    })
                ) as Self::Future
            },

            // ResponsesWithHeadersGet - GET /responses_with_headers
            &hyper::Method::GET if path.matched(paths::ID_RESPONSES_WITH_HEADERS) => {
                Box::new({
                        {{
                                Box::new(
                                    api_impl.responses_with_headers_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                ResponsesWithHeadersGetResponse::Success
                                                    {
                                                        body,
                                                        success_info,
                                                        object_header
                                                    }
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        HeaderName::from_static("success-info"),
                                                        header::IntoHeaderValue(success_info).into()
                                                    );
                                                    response.headers_mut().insert(
                                                        HeaderName::from_static("object-header"),
                                                        header::IntoHeaderValue(object_header).into()
                                                    );
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for RESPONSES_WITH_HEADERS_GET_SUCCESS"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                ResponsesWithHeadersGetResponse::PreconditionFailed
                                                    {
                                                        further_info,
                                                        failure_info
                                                    }
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(412).expect("Unable to turn 412 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        HeaderName::from_static("further-info"),
                                                        header::IntoHeaderValue(further_info).into()
                                                    );
                                                    response.headers_mut().insert(
                                                        HeaderName::from_static("failure-info"),
                                                        header::IntoHeaderValue(failure_info).into()
                                                    );
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // Rfc7807Get - GET /rfc7807
            &hyper::Method::GET if path.matched(paths::ID_RFC7807) => {
                Box::new({
                        {{
                                Box::new(
                                    api_impl.rfc7807_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                Rfc7807GetResponse::OK
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(204).expect("Unable to turn 204 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for RFC7807_GET_OK"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                Rfc7807GetResponse::NotFound
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(404).expect("Unable to turn 404 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/problem+json")
                                                            .expect("Unable to create Content-Type header for RFC7807_GET_NOT_FOUND"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                Rfc7807GetResponse::NotAcceptable
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(406).expect("Unable to turn 406 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/problem+xml")
                                                            .expect("Unable to create Content-Type header for RFC7807_GET_NOT_ACCEPTABLE"));
                                                    let body = serde_xml_rs::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // UntypedPropertyGet - GET /untyped_property
            &hyper::Method::GET if path.matched(paths::ID_UNTYPED_PROPERTY) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_object_untyped_props: Option<models::ObjectUntypedProps> = if !body.is_empty() {
                                    let deserializer = &mut serde_json::Deserializer::from_slice(&*body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_object_untyped_props) => param_object_untyped_props,
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                };

                                Box::new(
                                    api_impl.untyped_property_get(
                                            param_object_untyped_props,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                UntypedPropertyGetResponse::CheckThatUntypedPropertiesWorks
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter ObjectUntypedProps: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter ObjectUntypedProps"))),
                        }
                    })
                ) as Self::Future
            },

            // UuidGet - GET /uuid
            &hyper::Method::GET if path.matched(paths::ID_UUID) => {
                Box::new({
                        {{
                                Box::new(
                                    api_impl.uuid_get(
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                UuidGetResponse::DuplicateResponseLongText
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for UUID_GET_DUPLICATE_RESPONSE_LONG_TEXT"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            // XmlExtraPost - POST /xml_extra
            &hyper::Method::POST if path.matched(paths::ID_XML_EXTRA) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_duplicate_xml_object: Option<models::DuplicateXmlObject> = if !body.is_empty() {
                                    let deserializer = &mut serde_xml_rs::de::Deserializer::new_from_reader(&*body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_duplicate_xml_object) => param_duplicate_xml_object,
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                };

                                Box::new(
                                    api_impl.xml_extra_post(
                                            param_duplicate_xml_object,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                XmlExtraPostResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                },
                                                XmlExtraPostResponse::BadRequest
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter DuplicateXmlObject: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter DuplicateXmlObject"))),
                        }
                    })
                ) as Self::Future
            },

            // XmlOtherPost - POST /xml_other
            &hyper::Method::POST if path.matched(paths::ID_XML_OTHER) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_another_xml_object: Option<models::AnotherXmlObject> = if !body.is_empty() {
                                    let deserializer = &mut serde_xml_rs::de::Deserializer::new_from_reader(&*body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_another_xml_object) => param_another_xml_object,
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                };

                                Box::new(
                                    api_impl.xml_other_post(
                                            param_another_xml_object,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                XmlOtherPostResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                },
                                                XmlOtherPostResponse::BadRequest
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter AnotherXmlObject: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter AnotherXmlObject"))),
                        }
                    })
                ) as Self::Future
            },

            // XmlOtherPut - PUT /xml_other
            &hyper::Method::PUT if path.matched(paths::ID_XML_OTHER) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_string: Option<models::AnotherXmlArray> = if !body.is_empty() {
                                    let deserializer = &mut serde_xml_rs::de::Deserializer::new_from_reader(&*body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_string) => param_string,
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                };

                                Box::new(
                                    api_impl.xml_other_put(
                                            param_string,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                XmlOtherPutResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                },
                                                XmlOtherPutResponse::BadRequest
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter string: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter string"))),
                        }
                    })
                ) as Self::Future
            },

            // XmlPost - POST /xml
            &hyper::Method::POST if path.matched(paths::ID_XML) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_string: Option<models::XmlArray> = if !body.is_empty() {
                                    let deserializer = &mut serde_xml_rs::de::Deserializer::new_from_reader(&*body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_string) => param_string,
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                };

                                Box::new(
                                    api_impl.xml_post(
                                            param_string,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                XmlPostResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                },
                                                XmlPostResponse::BadRequest
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter string: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter string"))),
                        }
                    })
                ) as Self::Future
            },

            // XmlPut - PUT /xml
            &hyper::Method::PUT if path.matched(paths::ID_XML) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_xml_object: Option<models::XmlObject> = if !body.is_empty() {
                                    let deserializer = &mut serde_xml_rs::de::Deserializer::new_from_reader(&*body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_xml_object) => param_xml_object,
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                };

                                Box::new(
                                    api_impl.xml_put(
                                            param_xml_object,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                XmlPutResponse::OK
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                },
                                                XmlPutResponse::BadRequest
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter XmlObject: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter XmlObject"))),
                        }
                    })
                ) as Self::Future
            },

            // CreateRepo - POST /repos
            &hyper::Method::POST if path.matched(paths::ID_REPOS) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                Box::new(body.concat2()
                    .then(move |result| -> Self::Future {
                        match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_object_param: Option<models::ObjectParam> = if !body.is_empty() {
                                    let deserializer = &mut serde_json::Deserializer::from_slice(&*body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_object_param) => param_object_param,
                                        Err(e) => return Box::new(future::ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from(format!("Couldn't parse body parameter ObjectParam - doesn't match schema: {}", e)))
                                                        .expect("Unable to create Bad Request response for invalid body parameter ObjectParam due to schema"))),
                                    }
                                } else {
                                    None
                                };
                                let param_object_param = match param_object_param {
                                    Some(param_object_param) => param_object_param,
                                    None => return Box::new(future::ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from("Missing required body parameter ObjectParam"))
                                                        .expect("Unable to create Bad Request response for missing body parameter ObjectParam"))),
                                };

                                Box::new(
                                    api_impl.create_repo(
                                            param_object_param,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                CreateRepoResponse::Success
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for CREATE_REPO_SUCCESS"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                            },
                            Err(e) => Box::new(future::ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter ObjectParam: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter ObjectParam"))),
                        }
                    })
                ) as Self::Future
            },

            // Get RepoInfo - GET /repos/{repoId}
            &hyper::Method::GET if path.matched(paths::ID_REPOS_REPOID) => {
                // Path parameters
                let path: &str = &uri.path().to_string();
                let path_params =
                    paths::REGEX_REPOS_REPOID
                    .captures(&path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE REPOS_REPOID in set but failed match against \"{}\"", path, paths::REGEX_REPOS_REPOID.as_str())
                    );

                let param_repo_id = match percent_encoding::percent_decode(path_params["repoId"].as_bytes()).decode_utf8() {
                    Ok(param_repo_id) => match param_repo_id.parse::<String>() {
                        Ok(param_repo_id) => param_repo_id,
                        Err(e) => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter repoId: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter"))),
                    },
                    Err(_) => return Box::new(future::ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["repoId"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode")))
                };

                Box::new({
                        {{
                                Box::new(
                                    api_impl.get_repo_info(
                                            param_repo_id,
                                        &context
                                    ).then(move |result| {
                                        let mut response = Response::new(Body::empty());
                                        response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().to_string().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                Get RepoInfoResponse::OK
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for GET_REPO_INFO_OK"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        future::ok(response)
                                    }
                                ))
                        }}
                }) as Self::Future
            },

            _ if path.matched(paths::ID_CALLBACK_WITH_HEADER) => method_not_allowed(),
            _ if path.matched(paths::ID_COMPLEX_QUERY_PARAM) => method_not_allowed(),
            _ if path.matched(paths::ID_ENUM_IN_PATH_PATH_PARAM) => method_not_allowed(),
            _ if path.matched(paths::ID_MANDATORY_REQUEST_HEADER) => method_not_allowed(),
            _ if path.matched(paths::ID_MERGE_PATCH_JSON) => method_not_allowed(),
            _ if path.matched(paths::ID_MULTIGET) => method_not_allowed(),
            _ if path.matched(paths::ID_MULTIPLE_AUTH_SCHEME) => method_not_allowed(),
            _ if path.matched(paths::ID_OVERRIDE_SERVER) => method_not_allowed(),
            _ if path.matched(paths::ID_PARAMGET) => method_not_allowed(),
            _ if path.matched(paths::ID_READONLY_AUTH_SCHEME) => method_not_allowed(),
            _ if path.matched(paths::ID_REGISTER_CALLBACK) => method_not_allowed(),
            _ if path.matched(paths::ID_REPOS) => method_not_allowed(),
            _ if path.matched(paths::ID_REPOS_REPOID) => method_not_allowed(),
            _ if path.matched(paths::ID_REQUIRED_OCTET_STREAM) => method_not_allowed(),
            _ if path.matched(paths::ID_RESPONSES_WITH_HEADERS) => method_not_allowed(),
            _ if path.matched(paths::ID_RFC7807) => method_not_allowed(),
            _ if path.matched(paths::ID_UNTYPED_PROPERTY) => method_not_allowed(),
            _ if path.matched(paths::ID_UUID) => method_not_allowed(),
            _ if path.matched(paths::ID_XML) => method_not_allowed(),
            _ if path.matched(paths::ID_XML_EXTRA) => method_not_allowed(),
            _ if path.matched(paths::ID_XML_OTHER) => method_not_allowed(),
            _ => Box::new(future::ok(
                Response::builder().status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .expect("Unable to create Not Found response")
            )) as Self::Future
        }
    }
}

impl<T, C> Clone for Service<T, C> where T: Clone
{
    fn clone(&self) -> Self {
        Service {
            api_impl: self.api_impl.clone(),
            marker: self.marker.clone(),
        }
    }
}

/// Request parser for `Api`.
pub struct ApiRequestParser;
impl<T> RequestParser<T> for ApiRequestParser {
    fn parse_operation_id(request: &Request<T>) -> Result<&'static str, ()> {
        let path = paths::GLOBAL_REGEX_SET.matches(request.uri().path());
        match request.method() {
            // CallbackWithHeaderPost - POST /callback-with-header
            &hyper::Method::POST if path.matched(paths::ID_CALLBACK_WITH_HEADER) => Ok("CallbackWithHeaderPost"),
            // ComplexQueryParamGet - GET /complex-query-param
            &hyper::Method::GET if path.matched(paths::ID_COMPLEX_QUERY_PARAM) => Ok("ComplexQueryParamGet"),
            // EnumInPathPathParamGet - GET /enum_in_path/{path_param}
            &hyper::Method::GET if path.matched(paths::ID_ENUM_IN_PATH_PATH_PARAM) => Ok("EnumInPathPathParamGet"),
            // MandatoryRequestHeaderGet - GET /mandatory-request-header
            &hyper::Method::GET if path.matched(paths::ID_MANDATORY_REQUEST_HEADER) => Ok("MandatoryRequestHeaderGet"),
            // MergePatchJsonGet - GET /merge-patch-json
            &hyper::Method::GET if path.matched(paths::ID_MERGE_PATCH_JSON) => Ok("MergePatchJsonGet"),
            // MultigetGet - GET /multiget
            &hyper::Method::GET if path.matched(paths::ID_MULTIGET) => Ok("MultigetGet"),
            // MultipleAuthSchemeGet - GET /multiple_auth_scheme
            &hyper::Method::GET if path.matched(paths::ID_MULTIPLE_AUTH_SCHEME) => Ok("MultipleAuthSchemeGet"),
            // OverrideServerGet - GET /override-server
            &hyper::Method::GET if path.matched(paths::ID_OVERRIDE_SERVER) => Ok("OverrideServerGet"),
            // ParamgetGet - GET /paramget
            &hyper::Method::GET if path.matched(paths::ID_PARAMGET) => Ok("ParamgetGet"),
            // ReadonlyAuthSchemeGet - GET /readonly_auth_scheme
            &hyper::Method::GET if path.matched(paths::ID_READONLY_AUTH_SCHEME) => Ok("ReadonlyAuthSchemeGet"),
            // RegisterCallbackPost - POST /register-callback
            &hyper::Method::POST if path.matched(paths::ID_REGISTER_CALLBACK) => Ok("RegisterCallbackPost"),
            // RequiredOctetStreamPut - PUT /required_octet_stream
            &hyper::Method::PUT if path.matched(paths::ID_REQUIRED_OCTET_STREAM) => Ok("RequiredOctetStreamPut"),
            // ResponsesWithHeadersGet - GET /responses_with_headers
            &hyper::Method::GET if path.matched(paths::ID_RESPONSES_WITH_HEADERS) => Ok("ResponsesWithHeadersGet"),
            // Rfc7807Get - GET /rfc7807
            &hyper::Method::GET if path.matched(paths::ID_RFC7807) => Ok("Rfc7807Get"),
            // UntypedPropertyGet - GET /untyped_property
            &hyper::Method::GET if path.matched(paths::ID_UNTYPED_PROPERTY) => Ok("UntypedPropertyGet"),
            // UuidGet - GET /uuid
            &hyper::Method::GET if path.matched(paths::ID_UUID) => Ok("UuidGet"),
            // XmlExtraPost - POST /xml_extra
            &hyper::Method::POST if path.matched(paths::ID_XML_EXTRA) => Ok("XmlExtraPost"),
            // XmlOtherPost - POST /xml_other
            &hyper::Method::POST if path.matched(paths::ID_XML_OTHER) => Ok("XmlOtherPost"),
            // XmlOtherPut - PUT /xml_other
            &hyper::Method::PUT if path.matched(paths::ID_XML_OTHER) => Ok("XmlOtherPut"),
            // XmlPost - POST /xml
            &hyper::Method::POST if path.matched(paths::ID_XML) => Ok("XmlPost"),
            // XmlPut - PUT /xml
            &hyper::Method::PUT if path.matched(paths::ID_XML) => Ok("XmlPut"),
            // CreateRepo - POST /repos
            &hyper::Method::POST if path.matched(paths::ID_REPOS) => Ok("CreateRepo"),
            // Get RepoInfo - GET /repos/{repoId}
            &hyper::Method::GET if path.matched(paths::ID_REPOS_REPOID) => Ok("Get RepoInfo"),
            _ => Err(()),
        }
    }
}
