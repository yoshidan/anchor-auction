# anchor-auction

This is the [anchor](https://github.com/coral-xyz/anchor) implementation for [solana-auction](https://github.com/yoshidan/solana-auction).

## Run

Build the solana program
```
npm run build
```

Run validator on the localnet

```
npm run validator
```

Get 10 SOL by airdrop and deploy program for localnet.
```
npm run deploy
```

Run the auction. The specification is same as [solana-auction](https://github.com/yoshidan/solana-auction#Specification)

```
npm run test
```

### Tips
The solana-auction uses `Pubkey::Default` for `highestBidderPubkey` and `highestBidderFtTempPubkey` and `highestBidderFtReturningPubkey` as default.
```
┌────────────────────────────────┬────────────────────────────────────────────────┐
│            (index)             │                     value                      │
├────────────────────────────────┼────────────────────────────────────────────────┤
│         isInitialized          │                       1                        │
│        exhibitorPubkey         │ '56soKhmhDDx9A4BUqVZmr7ENqBxroFZmtrNTzw47S5Dr' │
│    exhibitingNftTempPubkey     │ '2whKYwM683zVirVitGW6UYugxdqbipMouMiFg3Ye8Vgb' │
│   exhibitorFtReceivingPubkey   │ '82eyhAL5iWqFaF2SEvWBA4TpB1NxVqdRsbEgJvjWUH6Y' │
│             price              │                      200                       │
│             endAt              │           '2022-06-18T01:28:40.000Z'           │
│      highestBidderPubkey       │       '11111111111111111111111111111111'       │
│   highestBidderFtTempPubkey    │       '11111111111111111111111111111111'       │
│ highestBidderFtReturningPubkey │       '11111111111111111111111111111111'       │
└────────────────────────────────┴────────────────────────────────────────────────┘
```

But the anchor-auction uses exhibitor's main account and token account.  
This is because the anchor checks account owner strictly.   
The owner of the `highestBidderPubkey`  and `highestBidderFtTempPubkey` must be TokenProgram because they are TokenAccount.
```
┌────────────────────────────────┬────────────────────────────────────────────────┐
│            (index)             │                     Values                     │
├────────────────────────────────┼────────────────────────────────────────────────┤
│        exhibitorPubkey         │ '4778RUtquRrrNxonH8mozrodB3CTLXj1XsJ3q2qjKZFj' │
│    exhibitingNftTempPubkey     │ 'Hf7SDqrqQjt9xAMzGkAh3h2w5KrodswMgNjKosqgmavU' │
│   exhibitorFtReceivingPubkey   │ '7D5V22HwrbkG2j9XDcccDAwhoA6KaCJ2w1F3ij5JfL1i' │
│             price              │                      202                       │
│             endAt              │           '2022-07-01T12:29:04.000Z'           │
│      highestBidderPubkey       │ 'CYCaQvggatfFkrq6Z6XP7PuBtjdjSAeBYndj3g1nocz2' │
│   highestBidderFtTempPubkey    │ '5R8o237M9e6zvUKxKtFPaNpqkXq9mFFthGfe5dqc8RMv' │
│ highestBidderFtReturningPubkey │ '6QW2hAmMcRr5L4s2Y2weTy21KXaxqMnnnRWhhJ4HiT6N' │
└────────────────────────────────┴────────────────────────────────────────────────┘
```