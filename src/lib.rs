mod processor;
mod instruction;
mod state;

use processor::process_instruction;
use solana_program::entrypoint;

entrypoint!(process_instruction);