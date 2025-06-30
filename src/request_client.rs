use lazy_static::lazy_static;
use reqwest::ClientBuilder;
use reqwest_middleware::{ClientBuilder as ClientWithMiddlewareBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

const UPLOAD_RETRY_COUNT: u32 = 3;

lazy_static! {
    pub static ref REQUEST_CLIENT: ClientWithMiddleware = ClientWithMiddlewareBuilder::new(
        ClientBuilder::new()
            .user_agent("codspeed-runner")
            .build()
            .unwrap()
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(UPLOAD_RETRY_COUNT)
    ))
    .build();
}
