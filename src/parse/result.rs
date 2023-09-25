use std::{
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
};

pub enum ParserResult<T, E> {
    Some(T),
    Err(E),
    Done,
}

impl<T, E> ParserResult<T, E> {
    pub fn to_res_opt(self) -> Result<Option<T>, E> {
        match self {
            ParserResult::Some(v) => Ok(Some(v)),
            ParserResult::Err(e) => Err(e),
            ParserResult::Done => Ok(None),
        }
    }
    pub fn to_opt_res(self) -> Option<Result<T, E>> {
        match self {
            ParserResult::Some(v) => Some(Ok(v)),
            ParserResult::Err(e) => Some(Err(e)),
            ParserResult::Done => None,
        }
    }
}

impl<T, E> From<Result<T, E>> for ParserResult<T, E> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(v) => Self::Some(v),
            Err(e) => Self::Err(e),
        }
    }
}
impl<T, E> From<Option<T>> for ParserResult<T, E> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => Self::Some(v),
            None => Self::Done,
        }
    }
}

impl<T, E> Try for ParserResult<T, E> {
    type Output = T;
    type Residual = Option<E>;

    fn from_output(output: Self::Output) -> Self {
        ParserResult::Some(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            ParserResult::Some(v) => ControlFlow::Continue(v),
            ParserResult::Err(e) => ControlFlow::Break(Some(e)),
            ParserResult::Done => ControlFlow::Break(None),
        }
    }
}

impl<T, E> FromResidual for ParserResult<T, E> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        match residual {
            Some(e) => Self::Err(e),
            None => Self::Done,
        }
    }
}

impl<T, E> FromResidual<Result<Infallible, E>> for ParserResult<T, E> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Ok(_) => unreachable!(),
            Err(e) => Self::Err(e),
        }
    }
}
