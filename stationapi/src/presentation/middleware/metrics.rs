use std::{
    task::{Context, Poll},
    time::Instant,
};

use http::Request;
use metrics::{counter, histogram};
use pin_project_lite::pin_project;
use tower::{Layer, Service};

/// Tower Layer that records gRPC request metrics (counters + latency histogram).
#[derive(Clone, Debug)]
pub struct GrpcMetricsLayer;

impl<S> Layer<S> for GrpcMetricsLayer {
    type Service = GrpcMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GrpcMetricsService { inner }
    }
}

/// Tower Service wrapper that instruments each request.
#[derive(Clone, Debug)]
pub struct GrpcMetricsService<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for GrpcMetricsService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = GrpcMetricsFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let path = req.uri().path();

        // Only record metrics for valid gRPC paths (e.g. /app.trainlcd.grpc.StationAPI/Method)
        // to avoid cardinality explosion from bot/scanner requests like /$(pwd)/netlify.toml
        let method = if path.starts_with("/app.trainlcd.grpc.")
            || path.starts_with("/grpc.health.")
            || path.starts_with("/grpc.reflection.")
        {
            path.to_owned()
        } else {
            "unknown".to_owned()
        };

        counter!("grpc_requests_started_total", "method" => method.clone()).increment(1);

        GrpcMetricsFuture {
            inner: self.inner.call(req),
            method,
            start: Instant::now(),
        }
    }
}

pin_project! {
    /// Future that records latency and completion metrics when the inner future resolves.
    pub struct GrpcMetricsFuture<F> {
        #[pin]
        inner: F,
        method: String,
        start: Instant,
    }
}

impl<F, Response, Error> std::future::Future for GrpcMetricsFuture<F>
where
    F: std::future::Future<Output = Result<Response, Error>>,
{
    type Output = Result<Response, Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                let elapsed = this.start.elapsed().as_secs_f64();
                let status = if result.is_ok() { "ok" } else { "error" };
                let method = this.method.clone();

                counter!("grpc_requests_total", "method" => method.clone(), "status" => status)
                    .increment(1);
                histogram!("grpc_request_duration_seconds", "method" => method, "status" => status)
                    .record(elapsed);

                Poll::Ready(result)
            }
        }
    }
}
