use controller::Controller;
use receiver::Receiver;
use std::fmt::Display;
use std::hash::Hash;
use std::net::ToSocketAddrs;
use tiny_http::{Response, Server};

/// an HTTP-based endpoint for viewing all registered metrics on a `Receiver`
pub struct HttpReporter<T> {
    server: Server,
    controller: Controller<T>,
}

impl<T: Eq + Hash + Send + Clone + Display> HttpReporter<T> {
    /// creates a new `HttpReporter` from the given `Receiver`, listening on the given address
    pub fn new<U: ToSocketAddrs>(receiver: &Receiver<T>, listen: U) -> HttpReporter<T> {
        let address = listen
            .to_socket_addrs()
            .expect("SocketAddr lookup failed")
            .next()
            .expect("SocketAddr resolved to empty set");

        let controller = receiver.get_controller();
        let server = Server::http(address).unwrap();

        HttpReporter {
            server: server,
            controller: controller,
        }
    }

    /// runs the HTTP server loop, this will block the calling thread until the process exists
    ///
    /// you should run this via `thread::spawn`
    pub fn run(&mut self) {
        for request in self.server.incoming_requests() {
            let response = match self.controller.get_meters() {
                Ok(meters) => {
                    let mut output = "".to_owned();
                    match request.url() {
                        "/vars" | "/metrics" => {
                            for (stat, value) in &meters.data {
                                output = output + &format!("{} {}\n", stat, value);
                            }
                            for (stat, value) in &meters.data_float {
                                output = output + &format!("{} {}\n", stat, value);
                            }
                        }
                        _ => {
                            output += "{";
                            for (stat, value) in &meters.data {
                                output = output + &format!("\"{}\":{},", stat, value);
                            }
                            for (stat, value) in &meters.data_float {
                                output = output + &format!("\"{}\":{},", stat, value);
                            }
                            if output.len() > 1 {
                                output.pop();
                            }
                            output += "}";
                        }
                    }

                    Response::from_string(output)
                }
                Err(_) => {
                    let response = Response::from_string("failed to read meters from receiver");
                    response.with_status_code(500)
                }
            };

            let _ = request.respond(response);
        }
    }
}
