#[test]
fn cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
    trycmd::TestCases::new().case("tests/cmd/*.toml");
}
