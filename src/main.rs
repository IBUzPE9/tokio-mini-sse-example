//#![deny(warnings)]
extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate tokio_minihttp;
extern crate tokio_service;
extern crate bytes;

use futures::{Future, Stream, Sink, future};

use tokio_io::AsyncRead;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

use tokio_minihttp::{Response, HttpCodec};

use std::{io, thread};
use std::time::Duration;

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let remote_addr = "127.0.0.1:7778".parse().unwrap();

    let listener = TcpListener::bind(&remote_addr, &handle).unwrap();

    let server = listener.incoming().for_each(move |(socket, _)| {
        let transport = socket.framed(HttpCodec);
        println!("get connection");
        transport.into_future()
            .map_err(|(e, _)| e)
            .and_then(|(req, transport)| {
                match req {
                    Some(req) => match req.path() {
                        "/" => {
                            Box::new(
                                tokio_io::io::write_all(transport.into_inner(), b"HTTP/1.1 200 OK\r\nDate: Tue, 20 Jun 2017 17:44:38 GMT\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n".as_ref())
                                    .and_then(|(stream,_)|{
                                        thread::sleep(Duration::from_secs(2));
                                        tokio_io::io::write_all(stream, b"event: userconnect\ndata: {\"username\": \"Frodo\", \"status\": \"take a trip\"}\n\n".as_ref())
                                    })
                                    .and_then(|(stream, _)| {
                                        println!("msg 1 sent ok");
                                        thread::sleep(Duration::from_secs(2));
                                        tokio_io::io::write_all(stream, b"event: userconnect\ndata: {\"username\": \"Gimli\", \"status\": \"take a trip\"}\n\n".as_ref())
                                    })
                                    .and_then(|(stream, _)| {
                                        println!("msg 2 sent ok");
                                        thread::sleep(Duration::from_secs(2));
                                        tokio_io::io::write_all(stream, b"event: userconnect\ndata: {\"username\": \"Legolas\", \"status\": \"take a trip\"}\n\n".as_ref())
                                    })                                                                    
                                    .map(|(_, _)| {println!("msg 3 sent ok");})
                            ) as Box<Future<Item=(), Error=io::Error>>
                        }
                        _ =>  {
                            let mut resp = Response::new();
                            resp.status_code(404, "Not Found");
                            resp.body("not found");
                            Box::new(transport.send(resp).map(|_| ())) as Box<Future<Item=(), Error=io::Error>>                                       
                        }
                    },
                    _ => {
                        println!("stream terminated");
                        Box::new(future::err(io::Error::from(io::ErrorKind::InvalidInput))) as Box<Future<Item=(), Error=io::Error>>
                    }
                }
            })
    });
    core.run(server).unwrap();
}
