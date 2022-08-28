

use std::fs;
use std::str::from_utf8;
use std::time::Duration;
use async_std::io::{Read, Write};

use async_std::prelude::*;
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_std::task::spawn;
use futures::stream::StreamExt;

// Adding async to the function declaration changes its return type
// from the unit type () to a type that implements Future<Output=()>.
// handle_Connection does not actually require an async_std::net::TcpStream.
// It requires any struct that implements async_std::io::REad, async_std::io::WRite, and market::Unpin
async fn handle_connection(mut stream: impl Read + Write + Unpin) {
    // Read the first 1024 bytes of data from the stream
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await.unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    // Respond with greetings or a 404,
    // depending on the data in the request
    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else if buffer.starts_with(sleep) {
        task::sleep(Duration::from_secs(5)).await;
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    }
    else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };
    let contents = fs::read_to_string(filename).unwrap();

    // Write response back to the stream,
    // and flush the stream to ensure the response is sent back to the client
    let response = format!("{}{}", status_line, contents);
    stream.write(response.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}

async fn async_concurrent() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();

    // The asynchronous version of TcpListener implements the Stream trait for listener.incoming()
    listener.incoming()
        // for_each_concurrent is implemented by the StreamExt trait in the futures crate
        .for_each_concurrent(None, |stream| async move {
            let stream = stream.unwrap();
            // As long as handle_connection does not block, a slow request will no longer prevent other requests from completing
            handle_connection(stream).await;
        }).await;
}

async fn async_parallel() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();

    listener.incoming()
        .for_each_concurrent(None, |stream| async move {
            let stream = stream.unwrap();
            // Because handle_connection is both Send and non-blocking,
            // it's safe to use with async_std::task::spawn.
            spawn(handle_connection(stream));
        }).await;
}

#[async_std::main]
pub async fn main() {
    async_concurrent().await;
}

#[cfg(test)]
mod tests {
    use std::cmp::min;
    use std::io::{IoSlice, IoSliceMut};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use super::*;
    use futures::io::Error;

    struct MockTcpStream {
        read_data: Vec<u8>,
        write_data: Vec<u8>
    }

    impl Read for MockTcpStream {
        fn poll_read(self: Pin<&mut Self>, _: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
            let size: usize = min(self.read_data.len(), buf.len());
            buf[..size].copy_from_slice(&self.read_data[..size]);
            Poll::Ready(Ok(size))
        }
    }

    impl Write for MockTcpStream {
        fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
            unsafe {
                self.get_unchecked_mut().write_data = Vec::from(buf);
            }
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    // To indicate that its location in memory can safely be moved.
    impl Unpin for MockTcpStream {}

    #[async_std::test]
    async fn test_handle_connection() {
        let input_bytes = b"GET / HTTP/1.1\r\n";
        let mut contents = vec![0u8; 1024];
        contents[..input_bytes.len()].clone_from_slice(input_bytes);
        let mut stream = MockTcpStream {
            read_data: contents,
            write_data: Vec::new(),
        };

        handle_connection(&mut stream).await;

        let expected_contents = fs::read_to_string("hello.html").unwrap();
        let expected_response = format!("HTTP/1.1 200 OK\r\n\r\n{}", expected_contents);
        assert!(stream.write_data.starts_with(expected_response.as_bytes()));
    }
}