use axum::{http::Request, middleware::Next, response::Response};
use tracing::Instrument;
use uuid::Uuid;

pub async fn logger<B>(request: Request<B>, next: Next<B>) -> Response {
    let trace_id = Uuid::new_v4().to_string();
    let span = tracing::info_span!("request", trace_id = tracing::field::Empty);
    span.record("trace_id", tracing::field::display(trace_id));

    next.run(request).instrument(span).await
}
