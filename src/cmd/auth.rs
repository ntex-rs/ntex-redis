use super::{Command, CommandError, Request, Response};

/// Create AUTH redis command
pub fn Auth<T>(password: T) -> AuthCommand
where
    Request: From<T>,
{
    AuthCommand(Request::Array(vec![
        Request::from_static("AUTH"),
        password.into(),
    ]))
}

pub struct AuthCommand(Request);

impl Command for AuthCommand {
    type Output = bool;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::String(val) => Ok(val == "OK"),
            _ => Ok(false),
        }
    }
}
