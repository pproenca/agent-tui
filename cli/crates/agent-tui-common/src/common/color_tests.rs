use super::*;

#[test]
fn test_colors_disabled() {
    let _ = NO_COLOR.set(true);
    assert_eq!(success("test"), "test");
    assert_eq!(error("test"), "test");
}
