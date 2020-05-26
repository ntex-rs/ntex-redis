use super::{Command, CommandError, Request, Response};

/// Create SET redis command
///
/// Command returns true if value is set
/// otherwise it returns false
pub fn Set<T, V>(key: T, value: V) -> SetCommand
where
    Request: From<T> + From<V>,
{
    SetCommand(Request::Array(vec![
        Request::from_bstatic(b"SET"),
        key.into(),
        value.into(),
    ]))
}

pub struct SetCommand(Request);

impl Command for SetCommand {
    type Output = bool;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(false),
            Response::String(string) => match string.as_ref() {
                "OK" => Ok(true),
                _ => Err(CommandError::Output(
                    "Unexpected value within String",
                    Response::String(string),
                )),
            },
            _ => Err(CommandError::Output("Unexpected value", val)),
        }
    }
}
