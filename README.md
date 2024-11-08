

Subscription Service (awarded 10 times): 
Set up a canister that collects recurring payments from an Ethereum account, using USDC or WETH on Sepolia. approve the canister’s Ethereum address for a certain amount and then let the canister call transferFrom regularly using a timer to deduct the subscription fee from your account.


// 1. Callister needs to know which address to target
// 2. (front) do an approve
// 3. On daily basis call 'transferFrom'