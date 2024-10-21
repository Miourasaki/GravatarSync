pub static DEFAULT_AVATAR: &[u8] = include_bytes!("../default.avif");


pub static RATING_MAP: [(i8, &'static str); 4] = [
    (0, "g"),
    (1, "pg"),
    (2, "r"),
    (3, "x")
];
