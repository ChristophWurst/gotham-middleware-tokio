extern crate futures;
extern crate gotham;
extern crate gotham_middleware_tokio;
extern crate hyper;
extern crate mime;
extern crate tokio_core;

use futures::{future, Future, Stream};
use gotham::handler::{HandlerFuture, NewHandlerService};
use gotham::router::builder::{build_router, DefineSingleRoute, DrawRoutes};
use gotham::http::response::create_response;
use gotham::middleware::pipeline::new_pipeline;
use gotham::router::route::dispatch::{finalize_pipeline_set, new_pipeline_set};
use gotham::router::Router;
use gotham::state::State;
use gotham_middleware_tokio::{TokioMiddleware, TokioMiddlewareData};
use hyper::Client;
use hyper::server::Http;
use hyper::StatusCode;
use tokio_core::reactor::{Core, Remote};

fn proxy_handler(state: State) -> Box<HandlerFuture> {
    let work = {
        let handle = state
            .borrow::<TokioMiddlewareData>()
            .handle()
            .handle()
            .unwrap();
        let client = Client::new(&handle);

        client
            .get("http://httpbin.org/get".parse().unwrap())
            .and_then(|res| {
                println!("Response: {}", res.status());
                future::ok(b"ui".to_vec())
            })
    };

    let work = work.then(|_| {
        let resp = create_response(&state, StatusCode::Ok, Some((vec![], mime::TEXT_PLAIN)));
        future::ok((state, resp))
    });

    Box::new(work)
}

fn build_app_router(handle: Remote) -> Router {
    let middleware = TokioMiddleware::new(handle);
    let pipelines = new_pipeline_set();
    let (pipelines, default) = pipelines.add(new_pipeline().add(middleware).build());
    let pipelines = finalize_pipeline_set(pipelines);
    let default_pipeline_chain = (default, ());

    build_router(default_pipeline_chain, pipelines, |route| {
        route.get("/").to(proxy_handler);
    })
}

fn main() {
    let addr = "127.0.0.1:7878".parse().unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let server = Http::new()
        .serve_addr_handle(
            &addr,
            &handle,
            NewHandlerService::new(build_app_router(core.remote())),
        )
        .unwrap();

    println!("Listening on http://{}", server.incoming_ref().local_addr());

    let handle2 = handle.clone();
    handle.spawn(
        server
            .for_each(move |conn| {
                handle2.spawn(
                    conn.map(|_| ())
                        .map_err(|err| println!("server error: {:?}", err)),
                );
                Ok(())
            })
            .map_err(|_| ()),
    );

    core.run(futures::future::empty::<(), ()>()).unwrap();
}
