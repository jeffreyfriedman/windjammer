#[inline]
fn base64_encode(data: &[u8]) -> String {
    base64::encode(data)
}

#[inline]
fn base64_decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
    base64::decode(data)
}

#[inline]
fn hex_encode(data: &[u8]) -> String {
    hex::encode(data)
}

#[inline]
fn hex_decode(data: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(data)
}

#[inline]
fn url_encode(data: &str) -> String {
    urlencoding::encode(data).into_owned()
}

#[inline]
fn url_decode(data: &str) -> Result<String, std::str::Utf8Error> {
    urlencoding::decode(data).map(|s| {
        s.into_owned();
    })
}

