use lazy_static::lazy_static;
use reqwest::ClientBuilder;
use reqwest_middleware::{ClientBuilder as ClientWithMiddlewareBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

const UPLOAD_RETRY_COUNT: u32 = 3;
const OIDC_RETRY_COUNT: u32 = 10;
const USER_AGENT: &str = "codspeed-runner";

lazy_static! {
    pub static ref REQUEST_CLIENT: ClientWithMiddleware = ClientWithMiddlewareBuilder::new(
        ClientBuilder::new()
            .user_agent(USER_AGENT)
            .build()
            .unwrap()
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(UPLOAD_RETRY_COUNT)
    ))
    .build();

    // Client without retry middleware for streaming uploads (can't be cloned)
    pub static ref STREAMING_CLIENT: reqwest::Client = ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    // Client with retry middleware for OIDC token requests
    pub static ref OIDC_CLIENT: ClientWithMiddleware = ClientWithMiddlewareBuilder::new(
        ClientBuilder::new()
            .user_agent(USER_AGENT)
            .build()
            .unwrap()
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(OIDC_RETRY_COUNT)
    ))
    .build();
}
