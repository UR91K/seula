use tonic::{Code, Status};

use crate::error::DatabaseError;

impl From<DatabaseError> for Status {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::NotFound(msg) => Status::new(Code::NotFound, msg),
            DatabaseError::InvalidOperation(msg) => Status::new(Code::InvalidArgument, msg),
            other => Status::new(Code::Internal, format!("Database error: {}", other)),
        }
    }
}
