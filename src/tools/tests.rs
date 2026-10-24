use super::*;
#[test]
fn conversions() {
    assert_eq!(
        convert(Tool::Md5, "hello", false).unwrap(),
        "5d41402abc4b2a76b9719d911017c592"
    );
    assert_eq!(
        convert(
            Tool::Base64,
            &convert(Tool::Base64, "你好", false).unwrap(),
            true
        )
        .unwrap(),
        "你好"
    );
    assert!(convert(Tool::Base64, "!!!", true).is_err());
    assert_eq!(convert(Tool::Chinese, "软件", false).unwrap(), "軟體");
    assert!(
        convert(Tool::Timestamp, "0", false)
            .unwrap()
            .contains("1970-01-01")
    );
    assert!(
        convert(Tool::Timestamp, "1970-01-01T00:00:00Z", true)
            .unwrap()
            .contains("秒：0")
    );
    assert!(convert(Tool::Timestamp, "9223372036854775807", false).is_err());
    assert!(convert(Tool::Json, "{", false).is_err());
    assert_eq!(
        convert(
            Tool::Url,
            &convert(Tool::Url, "你好 &", false).unwrap(),
            true
        )
        .unwrap(),
        "你好 &"
    );
}
