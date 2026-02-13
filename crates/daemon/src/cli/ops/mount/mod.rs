use clap::{Args, Subcommand};

pub mod add;
pub mod list;
pub mod remove;
pub mod set;
pub mod start;
pub mod stop;

use crate::cli::op::Op;

crate::command_enum! {
    (List, list::List),
    (Add, add::Add),
    (Remove, remove::Remove),
    (Start, start::Start),
    (Stop, stop::Stop),
    (Set, set::Set),
}

// Rename the generated Command to MountCommand for clarity
pub type MountCommand = Command;

#[derive(Args, Debug, Clone)]
pub struct Mount {
    #[command(subcommand)]
    pub command: MountCommand,
}

#[async_trait::async_trait]
impl Op for Mount {
    type Error = OpError;
    type Output = OpOutput;

    async fn execute(&self, ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        self.command.execute(ctx).await
    }
}
