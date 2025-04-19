# NFT-STAKING

### Setup Guide
- Clone the Project.
- Start the Local Validator ```solana-test-validator```
- Navigate to project directory ```cd nft-staking```
- Build the Project ```cargo build-sbf```
- Load the **Metaplex Program** to the Local Validator:
		- ```solana program dump -um metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s ./target/deploy/nft_staking_native.so```
		- ```solana-test-validator --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s ./target/deploy/nft_staking_native.so --reset```
- Build and Deploy the Program
		- ```cargo build-sbf```
		- ```solana program deploy ./target/deploy/nft_staking_native.so```
- Run the tests
		- Navigate to the client folder ```cd client```
		- Replace the ProgramID in ```index.test.ts```
		- Run ```bun test --timeout 60000``` in the terminal to run the tests.



### CONTRIBUTIONS
If you feel an issue or something needs to be fixed , please raise an Issue or a PR. Your contributions are welcomed most !! :pray: