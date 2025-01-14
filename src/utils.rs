use crate::default::RATING_MAP;

pub fn is_hex_string(s: &str) -> bool {
    // 检查字符串长度是否为32
    if s.len() != 32 {
        return false;
    }

    // 遍历每个字符，检查是否为有效的十六进制字符
    for c in s.chars() {
        if !c.is_digit(16) { // 检查字符是否是0-9或a-f或A-F
            return false;
        }
    }

    true
}



pub fn get_rating_from_value(_v:Option<String>) -> i8 {
    for &(k, v) in RATING_MAP.iter() {
        if _v == Some(v.to_string()) { return k; }
    }
    0
}

pub fn get_rating(_k:i8) -> &'static str {
    for &(k, v) in RATING_MAP.iter() {
        if _k == k { return v; }
    }
    "g"
}