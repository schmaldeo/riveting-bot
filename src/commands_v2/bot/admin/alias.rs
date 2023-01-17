use crate::commands_v2::prelude::*;
// use crate::utils::prelude::*;

/// Command: Manage guild command aliases.
#[derive(Default)]
pub struct Alias;

impl Command for Alias {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(ctx: Context, _req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }
}

/// Command: List guild command aliases.
#[derive(Default)]
pub struct List;

impl Command for List {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(ctx: Context, _req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }
}

/// Command: Get a guild command alias definition.
#[derive(Default)]
pub struct Get;

impl Command for Get {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(ctx: Context, _req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }
}

/// Command: Set a guild command alias definition.
#[derive(Default)]
pub struct Set;

impl Command for Set {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(ctx: Context, _req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }
}

/// Command: Remove a guild command alias definition.
#[derive(Default)]
pub struct Remove;

impl Command for Remove {
    type Data = Self;

    async fn uber(_ctx: Context, _data: Self::Data) -> CommandResult {
        todo!()
    }

    async fn classic(ctx: Context, _req: ClassicRequest) -> CommandResult {
        Self::uber(ctx, Default::default()).await
    }
}
