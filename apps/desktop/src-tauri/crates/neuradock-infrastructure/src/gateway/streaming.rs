pub(crate) fn is_relay_billing_sse_block(block: &[u8]) -> bool {
    block
        .windows(b"event: billing_summary".len())
        .any(|window| window == b"event: billing_summary")
        || block
            .windows(b"\"type\":\"billing_summary\"".len())
            .any(|window| window == b"\"type\":\"billing_summary\"")
}

pub(crate) fn take_sse_block(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let lf = buffer.windows(2).position(|window| window == b"\n\n");
    let crlf = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    let (offset, delimiter_len) = match (lf, crlf) {
        (Some(lf), Some(crlf)) if lf <= crlf => (lf, 2),
        (Some(_), Some(crlf)) => (crlf, 4),
        (Some(lf), None) => (lf, 2),
        (None, Some(crlf)) => (crlf, 4),
        (None, None) => return None,
    };
    Some(buffer.drain(..offset + delimiter_len).collect())
}
