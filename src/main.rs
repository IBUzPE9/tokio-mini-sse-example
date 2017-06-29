#![deny(warnings)]
extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate tokio_minihttp;
extern crate tokio_service;
extern crate bytes;

use futures::{Future, Stream, Sink, future, Poll};
use futures::future::{Loop, loop_fn};

use tokio_io::AsyncRead;
use tokio_io::io::write_all;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Timeout};
use tokio_minihttp::{Response, HttpCodec};

use std::io;
use std::io::Write;
use std::time::Duration;

enum Case<A,B,C,D>{A(A), B(B), C(C), D(D)}

impl<A,B,C,D,I,E> Future for Case<A,B,C,D>
where 
    A:Future<Item=I, Error=E>,
    B:Future<Item=I, Error=E>,
    C:Future<Item=I, Error=E>,
    D:Future<Item=I, Error=E>,
{
    type Item = I;
    type Error = E;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error>{
        match *self {
            Case::A(ref mut a) => a.poll(),
            Case::B(ref mut b) => b.poll(),
            Case::C(ref mut c) => c.poll(),
            Case::D(ref mut d) => d.poll(),
        }
    }
}

fn rnd(x:usize) -> usize{
    (7 * x + 13) % 10
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let rht = &handle;
    let remote_addr = "127.0.0.1:7778".parse().unwrap();

    let listener = TcpListener::bind(&remote_addr, &handle).unwrap();

    let server = listener.incoming().for_each(|(socket, _)| {
        socket.framed(HttpCodec).into_future()
            .map_err(|(e, _)| e)
            .and_then(|(req, transport)| 
                match req {
                    Some(req) => match req.path() {
                        "/events" => {
                            println!("request events");
                            Case::A(
                                write_all(transport.into_inner(), SSE_RESP)
                                    .and_then(|(stream,_)|
                                        loop_fn((stream, 9, Vec::new()), |(stream, cnt, mut buf)|
                                            Timeout::new(Duration::from_secs(2), rht).unwrap()
                                                .and_then(move |_|{
                                                    let port = stream.peer_addr().map(|addr| addr.port()).unwrap_or(42) as usize;
                                                    let char = rnd(cnt + port);
                                                    let deed = rnd(char + port);
                                                    buf.clear();
                                                    write!(buf, "event: userconnect\ndata: {{\"username\": \"{}\", \"status\": \"{}\"}}\n\n", LOTR_CHARS[char], LOTR_DEEDS[deed]).unwrap();
                                                    write_all(stream, buf)
                                                        .and_then(move |(stream, buf)|
                                                            if cnt != 0 {
                                                                Ok(Loop::Continue((stream, cnt-1, buf)))
                                                            }else{
                                                                Ok(Loop::Break(()))
                                                            }                                                    
                                                        )                                                    
                                                })  

                                        )
                                    )
                                    .or_else(|e| {println!("{:?}", e); Ok(())})
                            )
                        }
                        "/" => {
                            println!("request html");
                            let mut resp = Response::new();
                            resp.status_code(200, "Ok");
                            resp.body(HTML_PAGE);
                            Case::B(transport.send(resp).map(|_| ())
                                .or_else(|e| {println!("{:?}", e); Ok(())})
                            )                            
                        }
                        _ =>  {
                            println!("request is wrong");
                            let mut resp = Response::new();
                            resp.status_code(404, "Not Found");
                            resp.body("not found");
                            Case::C(transport.send(resp).map(|_| ())
                                .or_else(|e| {println!("{:?}", e); Ok(())})
                            )                                 
                        }
                    },
                    _ => Case::D(future::err(io::Error::from(io::ErrorKind::InvalidInput)))
                }
            )
    });
    core.run(server).unwrap();
}


static LOTR_CHARS:&[&str] = &["Frodo", "Bilbo", "Gimli", "Legolas", "Gandalf", "Sam", "Gollum", "Aragorn", "Boromir", "Galadriel"];
static LOTR_DEEDS:&[&str] = &["wounded by King Nazgul’s blade", "leaves the Ring behind", "is fascinated by Galadriel's beauty", "hits a flying Nazgul", "overthrows Saruman and breaks his staff", "returned to the Shire", "wanders through the lands of Middle-earth", "ruled Gondor", "dies heroically", "is independent, strong and wise"];

// there is no way to create response without `Content-Length` header with tokio-minihttp, so let's do a little trick
static SSE_RESP:&[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";

static HTML_PAGE:&str = r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8"> 
    <title>Rust tokio-minihttp example with SSE and hobbits</title>
  </head>
  <body>
    <h1>Rust tokio-minihttp example with SSE and hobbits</h1>

    <div id="sse-msg">
    </div>

    <script type="text/javascript">
      var evtSource = new EventSource("http://127.0.0.1:7778/events");

      evtSource.addEventListener("userconnect", function(e) {
          var sseMsgDiv = document.getElementById('sse-msg');
          const obj = JSON.parse(e.data);
          sseMsgDiv.innerHTML += '<p>' + obj.username + " " + obj.status + '</p>';
          console.log(obj);
      }, false);
    </script>
  </body>
</html>"#;