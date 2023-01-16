pub fn is_set(byte: u8, pos: u8) -> bool {
    byte & pos != pos
}
