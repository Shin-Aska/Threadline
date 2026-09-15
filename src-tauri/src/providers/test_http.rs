use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
pub(super) struct Request {
    pub headers: String,
    pub body: Vec<u8>,
}
pub(super) fn server(
    responses: Vec<(u16, &'static str)>,
) -> (String, std::thread::JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let task = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let deadline = Instant::now() + Duration::from_secs(5);
            let socket = loop {
                if let Ok((socket, _)) = listener.accept() {
                    break socket;
                }
                assert!(Instant::now() < deadline, "expected provider request");
                std::thread::sleep(Duration::from_millis(10));
            };
            socket.set_nonblocking(false).expect("blocking stream");
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("timeout");
            let mut reader = BufReader::new(socket);
            let mut headers = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).expect("header") > 0);
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().expect("length");
                    }
                }
                headers.push_str(&line);
            }
            let mut request_body = vec![0; length];
            reader.read_exact(&mut request_body).expect("body");
            requests.push(Request {
                headers,
                body: request_body,
            });
            write!(reader.get_mut(), "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).expect("response");
        }
        requests
    });
    (url, task)
}
