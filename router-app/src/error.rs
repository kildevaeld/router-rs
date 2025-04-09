use heather::HBoxError;

#[derive(Debug)]
pub struct Error {}

impl From<HBoxError<'static>> for Error {
    fn from(value: HBoxError<'static>) -> Self {
        todo!()
    }
}
