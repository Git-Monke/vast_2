use axum::Router;

#[tokio::main]
async fn main() {
    let app = Router::new();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port");
    axum::serve(listener, app).await.expect("Failed to serve");
}
