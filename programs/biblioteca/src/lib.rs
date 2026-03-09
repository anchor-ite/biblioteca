use anchor_lang::prelude::*;

declare_id!("45D5JdUitCeHKQ3EiDQuxrhxH5GxWPqkR7Q5W3CMaAhW");

#[program]
pub mod biblioteca {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
