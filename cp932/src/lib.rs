pub fn decode_lossy(bytes: &[u8]) -> String {
	encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}

#[test]
fn decodes() {
	assert_eq!(decode_lossy(&[0x83, 0x8D, 0x83, 0x43, 0x83, 0x68]), "ロイド");
	assert_eq!(decode_lossy(b"Lloyd"), "Lloyd");
}
