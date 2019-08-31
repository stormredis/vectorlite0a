use futures::{Future, Sink};

pub mod blackhole;
pub mod clickhouse;
pub mod console;
pub mod http;
pub mod tcp;
pub mod util;
pub mod vector;

use crate::Event;

pub type RouterSink = Box<dyn Sink<SinkItem = Event, SinkError = ()> + 'static + Send>;

pub type Healthcheck = Box<dyn Future<Item = (), Error = String> + Send>;
