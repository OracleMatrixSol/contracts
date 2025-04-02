use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("YOUR_PROGRAM_ID_HERE");

#[program]
pub mod oraclematrix {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, validation_threshold: u8) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.validation_threshold = validation_threshold; // Minimum votes for validation
        config.data_source_count = 0;
        config.validator_count = 0;
        Ok(())
    }

    pub fn register_data_source(
        ctx: Context<RegisterDataSource>,
        data_type: DataType,
        initial_reputation: u64,
    ) -> Result<()> {
        let data_source = &mut ctx.accounts.data_source;
        let config = &mut ctx.accounts.config;
        data_source.authority = ctx.accounts.authority.key();
        data_source.data_type = data_type;
        data_source.reputation = initial_reputation;
        data_source.id = config.data_source_count;
        data_source.active = true;
        config.data_source_count += 1;
        emit!(DataSourceRegistered {
            source_id: data_source.id,
            data_type,
        });
        Ok(())
    }

    pub fn submit_data(
        ctx: Context<SubmitData>,
        data: Vec<u8>,
        metadata: Vec<u8>,
        timestamp: i64,
    ) -> Result<()> {
        let data_entry = &mut ctx.accounts.data_entry;
        let data_source = &ctx.accounts.data_source;
        require!(data_source.active, ErrorCode::InactiveDataSource);
        require!(data.len() <= 1024, ErrorCode::DataTooLarge); // Arbitrary limit for now
        data_entry.source_id = data_source.id;
        data_entry.data = compress_data(&data); // Simple compression placeholder
        data_entry.metadata = metadata;
        data_entry.timestamp = timestamp;
        data_entry.validation_count = 0;
        data_entry.validated = false;
        data_entry.expiry = Clock::get()?.unix_timestamp + 86_400; // 1 day expiry
        emit!(DataSubmitted {
            source_id: data_source.id,
            data_hash: hash_data(&data),
        });
        Ok(())
    }

    pub fn validate_data(ctx: Context<ValidateData>, is_valid: bool) -> Result<()> {
        let data_entry = &mut ctx.accounts.data_entry;
        let validator = &mut ctx.accounts.validator;
        let config = &ctx.accounts.config;
        require!(validator.stake >= 1000, ErrorCode::InsufficientStake); // Minimum stake
        require!(
            data_entry.expiry > Clock::get()?.unix_timestamp,
            ErrorCode::DataExpired
        );
        require!(
            !validator.votes.contains(&data_entry.key()),
            ErrorCode::AlreadyVoted
        );

        validator.votes.push(data_entry.key());
        if is_valid {
            data_entry.validation_count += 1;
            validator.reputation += 1;
        } else {
            validator.reputation = validator.reputation.saturating_sub(1);
            if validator.reputation < 10 {
                validator.stake = validator.stake.saturating_sub(100); // Slashing
            }
        }

        if data_entry.validation_count >= config.validation_threshold {
            data_entry.validated = true;
            emit!(DataValidated {
                data_entry: data_entry.key(),
            });
        }
        Ok(())
    }

    pub fn stake_validator(ctx: Context<StakeValidator>, amount: u64) -> Result<()> {
        let validator = &mut ctx.accounts.validator;
        let config = &mut ctx.accounts.config;
        require!(amount >= 1000, ErrorCode::InsufficientStake);
        validator.authority = ctx.accounts.authority.key();
        validator.stake = amount;
        validator.reputation = 0;
        validator.id = config.validator_count;
        config.validator_count += 1;
        Ok(())
    }

    pub fn update_reputation(
        ctx: Context<UpdateReputation>,
        source_id: u64,
        new_reputation: u64,
    ) -> Result<()> {
        let data_source = &mut ctx.accounts.data_source;
        let governance = &ctx.accounts.governance;
        require!(governance.votes >= 3, ErrorCode::InsufficientVotes); // Governance threshold
        require!(data_source.id == source_id, ErrorCode::InvalidSource);
        data_source.reputation = new_reputation;
        if new_reputation < 10 {
            data_source.active = false;
        }
        emit!(ReputationUpdated {
            source_id,
            new_reputation,
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 1 + 8 + 8)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterDataSource<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
    #[account(init, payer = authority, space = 8 + 32 + 1 + 8 + 8 + 1)]
    pub data_source: Account<'info, DataSource>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SubmitData<'info> {
    #[account(init, payer = authority, space = 8 + 8 + 1024 + 256 + 8 + 1 + 8)]
    pub data_entry: Account<'info, DataEntry>,
    #[account(has_one = authority)]
    pub data_source: Account<'info, DataSource>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ValidateData<'info> {
    #[account(mut)]
    pub data_entry: Account<'info, DataEntry>,
    #[account(mut, has_one = authority)]
    pub validator: Account<'info, Validator>,
    #[account()]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct StakeValidator<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 8 + 8 + 8 + 1024)]
    pub validator: Account<'info, Validator>,
    #[account(mut)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateReputation<'info> {
    #[account(mut)]
    pub data_source: Account<'info, DataSource>,
    #[account(has_one = authority)]
    pub governance: Account<'info, Governance>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[account]
pub struct Config {
    pub authority: Pubkey,
    pub validation_threshold: u8,
    pub data_source_count: u64,
    pub validator_count: u64,
}

#[account]
pub struct DataSource {
    pub authority: Pubkey,
    pub data_type: DataType,
    pub id: u64,
    pub reputation: u64,
    pub active: bool,
}

#[account]
pub struct DataEntry {
    pub source_id: u64,
    pub data: Vec<u8>,
    pub metadata: Vec<u8>,
    pub timestamp: i64,
    pub validation_count: u8,
    pub validated: bool,
    pub expiry: i64,
}

#[account]
pub struct Validator {
    pub authority: Pubkey,
    pub stake: u64,
    pub reputation: u64,
    pub id: u64,
    pub votes: Vec<Pubkey>, // Tracks voted data entries
}

#[account]
pub struct Governance {
    pub authority: Pubkey,
    pub votes: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum DataType {
    Financial,
    Sports,
    Political,
    Weather,
    Custom,
}

#[event]
pub struct DataSourceRegistered {
    pub source_id: u64,
    pub data_type: DataType,
}

#[event]
pub struct DataSubmitted {
    pub source_id: u64,
    pub data_hash: [u8; 32],
}

#[event]
pub struct DataValidated {
    pub data_entry: Pubkey,
}

#[event]
pub struct ReputationUpdated {
    pub source_id: u64,
    pub new_reputation: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Data source is inactive")]
    InactiveDataSource,
    #[msg("Data size exceeds limit")]
    DataTooLarge,
    #[msg("Validator has insufficient stake")]
    InsufficientStake,
    #[msg("Data entry has expired")]
    DataExpired,
    #[msg("Validator has already voted")]
    AlreadyVoted,
    #[msg("Insufficient governance votes")]
    InsufficientVotes,
    #[msg("Invalid data source ID")]
    InvalidSource,
}

// Placeholder for data compression (to be implemented off-chain or with a custom algorithm)
fn compress_data(data: &Vec<u8>) -> Vec<u8> {
    data.clone() // Replace with actual compression logic
}

// Placeholder for data hashing
fn hash_data(data: &Vec<u8>) -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash[..data.len().min(32)].copy_from_slice(&data[..32.min(data.len())]);
    hash // Replace with proper hashing (e.g., SHA-256)
}