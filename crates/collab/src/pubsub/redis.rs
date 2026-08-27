use async_trait::async_trait;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader},
    net::TcpStream,
};
use url::Url;

use super::subscription_bus::{FanoutTransport, SubscriptionBusError};

const REDIS_FANOUT_CHANNEL: &str = "zed.collaboration.message.v1";
const MAX_REDIS_FRAME_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct RedisFanoutTransport {
    endpoint: Url,
}

impl RedisFanoutTransport {
    pub fn new(endpoint: &str) -> Result<Self, SubscriptionBusError> {
        let endpoint = Url::parse(endpoint).map_err(|_| SubscriptionBusError::InvalidRequest)?;
        if endpoint.scheme() != "redis"
            || endpoint.host_str().is_none()
            || endpoint.path().trim_matches('/').parse::<u16>().is_err()
                && !endpoint.path().trim_matches('/').is_empty()
        {
            return Err(SubscriptionBusError::InvalidRequest);
        }
        Ok(Self { endpoint })
    }

    pub async fn subscribe(
        &self,
        mut on_envelope: impl FnMut(Vec<u8>) + Send,
    ) -> Result<(), SubscriptionBusError> {
        let mut stream = self.connect().await?;
        authenticate(&mut stream, &self.endpoint).await?;
        write_command(
            &mut stream,
            &[b"SUBSCRIBE", REDIS_FANOUT_CHANNEL.as_bytes()],
        )
        .await?;
        let mut reader = BufReader::new(stream);
        read_resp_array(&mut reader).await?;
        loop {
            let frame = read_resp_array(&mut reader).await?;
            if frame.len() == 3 && frame.first().is_some_and(|value| value == b"message") {
                if frame[2].len() > MAX_REDIS_FRAME_BYTES {
                    return Err(SubscriptionBusError::Backpressure);
                }
                on_envelope(frame[2].clone());
            }
        }
    }

    async fn connect(&self) -> Result<TcpStream, SubscriptionBusError> {
        let host = self
            .endpoint
            .host_str()
            .ok_or(SubscriptionBusError::InvalidRequest)?;
        let port = self.endpoint.port().unwrap_or(6379);
        TcpStream::connect((host, port))
            .await
            .map_err(|_| SubscriptionBusError::Unavailable)
    }
}

#[async_trait]
impl FanoutTransport for RedisFanoutTransport {
    async fn publish(&self, encoded_envelope: Vec<u8>) -> Result<(), SubscriptionBusError> {
        if encoded_envelope.len() > MAX_REDIS_FRAME_BYTES {
            return Err(SubscriptionBusError::InvalidRequest);
        }
        let mut stream = self.connect().await?;
        authenticate(&mut stream, &self.endpoint).await?;
        write_command(
            &mut stream,
            &[
                b"PUBLISH",
                REDIS_FANOUT_CHANNEL.as_bytes(),
                &encoded_envelope,
            ],
        )
        .await?;
        let mut reader = BufReader::new(stream);
        read_resp_integer(&mut reader).await?;
        Ok(())
    }
}

async fn authenticate(stream: &mut TcpStream, endpoint: &Url) -> Result<(), SubscriptionBusError> {
    let password = endpoint.password();
    let username = endpoint.username();
    let Some(password) = password else {
        return Ok(());
    };
    if username.is_empty() {
        write_command(stream, &[b"AUTH", password.as_bytes()]).await?;
    } else {
        write_command(stream, &[b"AUTH", username.as_bytes(), password.as_bytes()]).await?;
    }
    let mut reader = BufReader::new(stream);
    read_resp_simple(&mut reader).await?;
    Ok(())
}

async fn write_command(
    stream: &mut TcpStream,
    parts: &[&[u8]],
) -> Result<(), SubscriptionBusError> {
    stream
        .write_all(format!("*{}\r\n", parts.len()).as_bytes())
        .await
        .map_err(|_| SubscriptionBusError::Unavailable)?;
    for part in parts {
        stream
            .write_all(format!("${}\r\n", part.len()).as_bytes())
            .await
            .map_err(|_| SubscriptionBusError::Unavailable)?;
        stream
            .write_all(part)
            .await
            .map_err(|_| SubscriptionBusError::Unavailable)?;
        stream
            .write_all(b"\r\n")
            .await
            .map_err(|_| SubscriptionBusError::Unavailable)?;
    }
    stream
        .flush()
        .await
        .map_err(|_| SubscriptionBusError::Unavailable)
}

async fn read_line(
    reader: &mut (impl tokio::io::AsyncBufRead + Unpin),
) -> Result<Vec<u8>, SubscriptionBusError> {
    let mut line = Vec::new();
    reader
        .read_until(b'\n', &mut line)
        .await
        .map_err(|_| SubscriptionBusError::Unavailable)?;
    if line.len() < 2 || !line.ends_with(b"\r\n") || line.len() > MAX_REDIS_FRAME_BYTES {
        return Err(SubscriptionBusError::Unavailable);
    }
    line.truncate(line.len() - 2);
    Ok(line)
}

async fn read_resp_simple(
    reader: &mut (impl tokio::io::AsyncBufRead + Unpin),
) -> Result<(), SubscriptionBusError> {
    let line = read_line(reader).await?;
    if line.first() == Some(&b'+') {
        Ok(())
    } else {
        Err(SubscriptionBusError::Unavailable)
    }
}

async fn read_resp_integer(
    reader: &mut (impl tokio::io::AsyncBufRead + Unpin),
) -> Result<i64, SubscriptionBusError> {
    let line = read_line(reader).await?;
    if line.first() != Some(&b':') {
        return Err(SubscriptionBusError::Unavailable);
    }
    std::str::from_utf8(&line[1..])
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or(SubscriptionBusError::Unavailable)
}

async fn read_resp_array(
    reader: &mut (impl tokio::io::AsyncBufRead + Unpin),
) -> Result<Vec<Vec<u8>>, SubscriptionBusError> {
    let header = read_line(reader).await?;
    if header.first() != Some(&b'*') {
        return Err(SubscriptionBusError::Unavailable);
    }
    let length = std::str::from_utf8(&header[1..])
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|length| *length <= 16)
        .ok_or(SubscriptionBusError::Unavailable)?;
    let mut parts = Vec::with_capacity(length);
    for _ in 0..length {
        let bulk_header = read_line(reader).await?;
        match bulk_header.first() {
            Some(b'$') => {
                let bulk_length = std::str::from_utf8(&bulk_header[1..])
                    .ok()
                    .and_then(|value| value.parse::<usize>().ok())
                    .filter(|length| *length <= MAX_REDIS_FRAME_BYTES)
                    .ok_or(SubscriptionBusError::Unavailable)?;
                let mut value = vec![0; bulk_length];
                reader
                    .read_exact(&mut value)
                    .await
                    .map_err(|_| SubscriptionBusError::Unavailable)?;
                let mut terminator = [0; 2];
                reader
                    .read_exact(&mut terminator)
                    .await
                    .map_err(|_| SubscriptionBusError::Unavailable)?;
                if terminator != *b"\r\n" {
                    return Err(SubscriptionBusError::Unavailable);
                }
                parts.push(value);
            }
            Some(b':') | Some(b'+') => parts.push(bulk_header[1..].to_vec()),
            _ => return Err(SubscriptionBusError::Unavailable),
        }
    }
    Ok(parts)
}
