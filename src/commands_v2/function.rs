use std::pin::Pin;
use std::sync::Arc;

use derive_more::{IsVariant, Unwrap};
use futures::Future;

use crate::commands_v2::request::{ClassicRequest, MessageRequest, SlashRequest, UserRequest};
use crate::commands_v2::CommandResult;
use crate::Context;

// use crate::utils::prelude::*;

pub mod mock {
    use super::*;
    use crate::commands_v2::request::{ClassicRequest, MessageRequest, SlashRequest, UserRequest};
    use crate::commands_v2::{CommandResult, Response};

    pub async fn classic(_ctx: Context, req: ClassicRequest) -> CommandResult {
        println!("CLASSIC REQ: {req:#?}");
        Ok(Response::Clear)
    }

    pub async fn slash(_ctx: Context, req: SlashRequest) -> CommandResult {
        println!("SLASH REQ: {req:#?}");
        Ok(Response::Clear)
    }

    pub async fn message(_ctx: Context, req: MessageRequest) -> CommandResult {
        println!("MESSAGE REQ: {req:#?}");
        Ok(Response::Clear)
    }

    pub async fn user(_ctx: Context, req: UserRequest) -> CommandResult {
        println!("USER REQ: {req:#?}");
        Ok(Response::Clear)
    }
}

/// Non-generic return type for async command functions.
pub type CallFuture = Pin<Box<dyn Future<Output = CommandResult> + Send>>;

macro_rules! function_trait {
    ( $( $request:ty => $var:path )* ) => {
        $(
            impl<F, Fut> Callable<$request> for F
            where
                F: Fn(Context, $request) -> Fut + Send + Sync + 'static,
                Fut: Future<Output = CommandResult> + Send + 'static,
            {
                fn call(&self, ctx: Context, req: $request) -> CallFuture {
                    Box::pin((self)(ctx, req))
                }

                fn into_shared(self) -> Arc<dyn Callable<$request>> {
                    Arc::new(self)
                }
            }

            impl Callable<$request> for Arc<dyn Callable<$request>> {
                fn call(&self, ctx: Context, req: $request) -> CallFuture {
                    (**self).call(ctx, req)
                }

                fn into_shared(self) -> Arc<dyn Callable<$request>> {
                    self
                }
            }

            impl<T> IntoFunction<$request> for T
            where
                T: Callable<$request>,
            {
                fn into_function(self) -> Function {
                    $var(self.into_shared())
                }
            }
        )*
    }
}

function_trait! {
    // Function that can handle basic text command.
    ClassicRequest => Function::Classic
    // Function that can handle interactive text command.
    SlashRequest => Function::Slash
    // Function that can handle GUI-based message command.
    MessageRequest => Function::Message
    // Function that can handle GUI-based user command.
    UserRequest => Function::User
}

pub trait ClassicFunction = Callable<ClassicRequest>;
pub trait SlashFunction = Callable<SlashRequest>;
pub trait MessageFunction = Callable<MessageRequest>;
pub trait UserFunction = Callable<UserRequest>;

/// Trait for functions that can be called with a generic request.
pub trait Callable<R>: Send + Sync {
    fn call(&self, ctx: Context, req: R) -> CallFuture;
    fn into_shared(self) -> Arc<dyn Callable<R>>;
}

/// Trait for converting something callable into a specific supported type.
pub trait IntoFunction<R> {
    fn into_function(self) -> Function;
}

/// Supported function types.
#[derive(Clone, Unwrap, IsVariant)]
pub enum Function {
    Classic(Arc<dyn Callable<ClassicRequest>>),
    Slash(Arc<dyn Callable<SlashRequest>>),
    Message(Arc<dyn Callable<MessageRequest>>),
    User(Arc<dyn Callable<UserRequest>>),
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Classic(_) => "Function::Classic(_)",
            Self::Slash(_) => "Function::Slash(_)",
            Self::Message(_) => "Function::Message(_)",
            Self::User(_) => "Function::User(_)",
        };
        writeln!(f, "{text}")
    }
}
