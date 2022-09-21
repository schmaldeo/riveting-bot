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
    ( $( $( #[$trait_meta:meta] )* $v:vis trait $fn_trait:ident { $request:ty } => $var:path )* ) => {
        $(
            $( #[$trait_meta] )*
            $v trait $fn_trait: Send + Sync {
                fn call(&self, ctx: Context, req: $request) -> CallFuture;
                fn into_shared(self) -> Arc<dyn $fn_trait>;
            }

            impl<F, Fut> $fn_trait for F
            where
                F: Fn(Context, $request) -> Fut + Send + Sync + 'static,
                Fut: Future<Output = CommandResult> + Send + 'static,
            {
                fn call(&self, ctx: Context, req: $request) -> CallFuture {
                    Box::pin((self)(ctx, req))
                }

                fn into_shared(self) -> Arc<dyn $fn_trait> {
                    Arc::new(self)
                }
            }

            impl $fn_trait for Arc<dyn $fn_trait> {
                fn call(&self, ctx: Context, req: $request) -> CallFuture {
                    (**self).call(ctx, req)
                }

                fn into_shared(self) -> Arc<dyn $fn_trait> {
                    self
                }
            }

            impl<T> IntoFunction<$request> for T
            where
                T: $fn_trait,
            {
                fn into_function(self) -> Function {
                    $var(self.into_shared())
                }
            }
        )*
    }
}

function_trait! {
    #[doc = "Function that can handle basic text command."]
    pub trait ClassicFunction { ClassicRequest } => Function::Classic

    #[doc = "Function that can handle interactive text command."]
    pub trait SlashFunction { SlashRequest } => Function::Slash

    #[doc = "Function that can handle GUI-based message command."]
    pub trait MessageFunction { MessageRequest } => Function::Message

    #[doc = "Function that can handle GUI-based user command."]
    pub trait UserFunction { UserRequest } => Function::User
}

/// Trait for converting something callable into a specific supported type.
pub trait IntoFunction<R> {
    fn into_function(self) -> Function;
}

/// Supported function types.
#[derive(Clone, Unwrap, IsVariant)]
pub enum Function {
    Classic(Arc<dyn ClassicFunction>),
    Slash(Arc<dyn SlashFunction>),
    Message(Arc<dyn MessageFunction>),
    User(Arc<dyn UserFunction>),
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
