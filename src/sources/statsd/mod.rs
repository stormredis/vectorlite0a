use crate::{topology::config::GlobalOptions, Event};
use futures::{future, sync::mpsc, Future, Sink, Stream};
use parser::parse;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::{
    self,
    codec::BytesCodec,
    net::{UdpFramed, UdpSocket},
};
use tracing::field;

mod parser;

#[derive(Deserialize, Serialize, Debug)]
struct StatsdConfig {
    address: SocketAddr,
}

#[typetag::serde(name = "statsd")]
impl crate::topology::config::SourceConfig for StatsdConfig {
    fn build(
        &self,
        _name: &str,
        _globals: &GlobalOptions,
        out: mpsc::Sender<Event>,
    ) -> Result<super::Source, String> {
        Ok(statsd(self.address, out))
    }

    fn output_type(&self) -> crate::topology::config::DataType {
        crate::topology::config::DataType::Metric
    }
}

fn statsd(addr: SocketAddr, out: mpsc::Sender<Event>) -> super::Source {
    let out = out.sink_map_err(|e| error!("error sending metric: {:?}", e));

    Box::new(
        future::lazy(move || {
            let socket = UdpSocket::bind(&addr).expect("failed to bind to udp listener socket");

            info!(
                message = "listening.",
                addr = &field::display(addr),
                r#type = "udp"
            );

            future::ok(socket)
        })
        .and_then(|socket| {
            let metrics_in = UdpFramed::new(socket, BytesCodec::new())
                .map(|(bytes, _sock)| {
                    let packet = String::from_utf8_lossy(bytes.as_ref());
                    let metrics = packet
                        .lines()
                        .map(parse)
                        .filter_map(|res| res.map_err(|e| error!("{}", e)).ok())
                        .map(Event::Metric)
                        .collect::<Vec<_>>();
                    futures::stream::iter_ok::<_, std::io::Error>(metrics)
                })
                .flatten()
                .map_err(|e| error!("error reading datagram: {:?}", e));

            metrics_in.forward(out).map(|_| info!("finished sending"))
        }),
    )
}
