pub trait ResultTestExt<T> {
    fn or_panic(self) -> T;
}

impl<T, E: std::fmt::Display> ResultTestExt<T> for Result<T, E> {
    fn or_panic(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("test helper expected success: {err}"),
        }
    }
}
