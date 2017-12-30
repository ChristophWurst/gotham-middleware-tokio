extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate tokio_core;

use std::io;
use std::panic::AssertUnwindSafe;

use gotham::handler::HandlerFuture;
use gotham::middleware::{Middleware, NewMiddleware};
use gotham::state::State;
use tokio_core::reactor::Remote;

pub struct TokioMiddleware {
    handle: AssertUnwindSafe<Remote>,
}

impl TokioMiddleware {
    pub fn new(handle: Remote) -> Self {
        TokioMiddleware {
            handle: AssertUnwindSafe(handle),
        }
    }
}

impl Middleware for TokioMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture> + 'static,
        Self: Sized,
    {
        state.put(TokioMiddlewareData::new(self.handle.clone()));

        chain(state)
    }
}

impl NewMiddleware for TokioMiddleware {
    type Instance = TokioMiddleware;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(TokioMiddleware {
            handle: AssertUnwindSafe(self.handle.clone()),
        })
    }
}

#[derive(StateData)]
pub struct TokioMiddlewareData {
    handle: Remote,
}

impl TokioMiddlewareData {
    pub fn new(remote: Remote) -> Self {
        TokioMiddlewareData { handle: remote }
    }
}

impl TokioMiddlewareData {
    pub fn handle(&self) -> &Remote {
        &self.handle
    }
}
