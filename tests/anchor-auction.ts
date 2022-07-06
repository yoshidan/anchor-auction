import * as anchor from '@project-serum/anchor';
import {AnchorProvider, Program} from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import {AnchorAuction} from '../target/types/anchor_auction';
import {SystemProgram, Connection, LAMPORTS_PER_SOL, PublicKey, Transaction} from '@solana/web3.js';
import {
    AccountLayout,
    AuthorityType,
    createAccount,
    createInitializeAccountInstruction,
    createMint,
    mintTo,
    setAuthority,
    TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import * as assert from "assert";

describe('anchor-auction', () => {
    const connection = new Connection("http://localhost:8899", "confirmed");
    const options = AnchorProvider.defaultOptions();
    const wallet = NodeWallet.local();
    const provider = new AnchorProvider(connection, wallet, options);

    anchor.setProvider(provider);
    const program = anchor.workspace.AnchorAuction as Program<AnchorAuction>;

    const payerAccount = wallet.payer
    const exhibitorAccount = anchor.web3.Keypair.generate();
    const bidder1Account = anchor.web3.Keypair.generate();
    const bidder2Account = anchor.web3.Keypair.generate();
    const escrowAccount = anchor.web3.Keypair.generate();

    let nftMintPubkey: PublicKey
    let ftMintPubkey: PublicKey
    let exhibitorNftTokenAccountPubkey: PublicKey
    let exhibitorFtTokenAccountPubkey: PublicKey
    let bidder1FtTokenAccountPubkey: PublicKey
    let bidder2FtTokenAccountPubkey: PublicKey
    it("Setup", async () => {
        await connection.requestAirdrop(exhibitorAccount.publicKey, LAMPORTS_PER_SOL * 2);
        await connection.requestAirdrop(bidder1Account.publicKey, LAMPORTS_PER_SOL * 2);
        await connection.requestAirdrop(bidder2Account.publicKey, LAMPORTS_PER_SOL * 2);
        nftMintPubkey = await createMint(connection, payerAccount, payerAccount.publicKey, null, 0, undefined, undefined, TOKEN_PROGRAM_ID);
        console.log(`Created NFT ${nftMintPubkey}`)
        exhibitorNftTokenAccountPubkey = await createAccount(connection, payerAccount, nftMintPubkey, exhibitorAccount.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
        await mintTo(connection, payerAccount, nftMintPubkey, exhibitorNftTokenAccountPubkey, payerAccount, 1, [], undefined, TOKEN_PROGRAM_ID);
        await setAuthority(connection, payerAccount, nftMintPubkey, payerAccount, AuthorityType.MintTokens, null);

        ftMintPubkey = await createMint(connection, payerAccount, payerAccount.publicKey, null, 0, undefined, undefined, TOKEN_PROGRAM_ID);
        console.log(`Created FT ${ftMintPubkey}`)
        exhibitorFtTokenAccountPubkey = await createAccount(connection, payerAccount, ftMintPubkey, exhibitorAccount.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
        await mintTo(connection, payerAccount, ftMintPubkey, exhibitorFtTokenAccountPubkey, payerAccount, 500, [], undefined, TOKEN_PROGRAM_ID);
        bidder1FtTokenAccountPubkey = await createAccount(connection, payerAccount, ftMintPubkey, bidder1Account.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
        await mintTo(connection, payerAccount, ftMintPubkey, bidder1FtTokenAccountPubkey, payerAccount, 500, [], undefined, TOKEN_PROGRAM_ID);
        bidder2FtTokenAccountPubkey = await createAccount(connection, payerAccount, ftMintPubkey, bidder2Account.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
        await mintTo(connection, payerAccount, ftMintPubkey, bidder2FtTokenAccountPubkey, payerAccount, 500, [], undefined, TOKEN_PROGRAM_ID);

        // sleep to allow time to update
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const data = {
            exhibitor: {
                "Wallet Pubkey": exhibitorAccount.publicKey.toBase58(),
                FT: await getTokenBalance(exhibitorFtTokenAccountPubkey, connection),
                "FT(NAO) Account PubKey": exhibitorFtTokenAccountPubkey.toBase58(),
                NFT: await getTokenBalance(exhibitorNftTokenAccountPubkey, connection),
                "NFT(X) Account PubKey": exhibitorNftTokenAccountPubkey.toBase58(),
            },
            bidder1: {
                "Wallet Pubkey": bidder1Account.publicKey.toBase58(),
                FT: await getTokenBalance(bidder1FtTokenAccountPubkey, connection),
                "FT(NAO) Account PubKey": bidder1FtTokenAccountPubkey.toBase58(),
                NFT: 0,
                "NFT(X) Account PubKey": "",
            },
            bidder2: {
                "Wallet Pubkey": bidder2Account.publicKey.toBase58(),
                FT: await getTokenBalance(bidder2FtTokenAccountPubkey, connection),
                "FT(NAO) Account PubKey": bidder2FtTokenAccountPubkey.toBase58(),
                NFT: 0,
                "NFT(X) Account PubKey": "",
            },
        };
        console.table(data);
    });

    let exhibitorNftTempAccount = anchor.web3.Keypair.generate();
    const initialPrice = 200
    const duration = 10

    // transaction fee payer is local wallet
    it("Exhibit", async () => {
        const signature = await program.rpc.exhibit(
            new anchor.BN(initialPrice),
            new anchor.BN(duration),
            {
                accounts: {
                    exhibitor: exhibitorAccount.publicKey,
                    exhibitorNftTokenAccount: exhibitorNftTokenAccountPubkey,
                    exhibitorNftTempAccount: exhibitorNftTempAccount.publicKey,
                    exhibitorFtReceivingAccount: exhibitorFtTokenAccountPubkey,
                    escrowAccount: escrowAccount.publicKey,
                    clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                instructions: [
                    ...await accountInstructions(connection, nftMintPubkey, exhibitorNftTempAccount.publicKey, exhibitorAccount.publicKey),
                    await program.account.auction.createInstruction(escrowAccount),
                ],
                signers: [exhibitorAccount, exhibitorNftTempAccount, escrowAccount]
            }
        );
        console.log(`exhibit tx = ${signature}`)

        await new Promise((resolve) => setTimeout(resolve, 1500));

        await logAuction(connection, escrowAccount.publicKey, program)
        assert.equal(await getTokenBalance(exhibitorNftTokenAccountPubkey, connection), 0)
        assert.equal(await getTokenBalance(exhibitorNftTempAccount.publicKey, connection), 1)
    })

    const bidder = async function (price: number, mintPubkey: PublicKey, bidder: anchor.web3.Keypair, bidderFtPubkey: PublicKey) {
        const bidderFtTempAccountKeypair = anchor.web3.Keypair.generate()
        const auction = await program.account.auction.fetch(escrowAccount.publicKey)
        const pda = await PublicKey.findProgramAddress([Buffer.from("escrow")], program.programId);
        const signature = await program.rpc.bid(
            new anchor.BN(price),
            {
                accounts: {
                    bidder: bidder.publicKey,
                    bidderFtTempAccount: bidderFtTempAccountKeypair.publicKey,
                    bidderFtAccount: bidderFtPubkey,
                    highestBidder: auction.highestBidderPubkey ,
                    highestBidderFtTempAccount: auction.highestBidderFtTempPubkey,
                    highestBidderFtReturningAccount: auction.highestBidderFtReturningPubkey,
                    escrowAccount: escrowAccount.publicKey,
                    clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
                    pda: pda[0],
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                instructions: [
                    ...await accountInstructions(connection, mintPubkey, bidderFtTempAccountKeypair.publicKey, bidder.publicKey),
                ],
                signers: [bidder,bidderFtTempAccountKeypair]
            }
        );
        console.log(`bidder tx = ${signature}`)
        await new Promise((resolve) => setTimeout(resolve, 1000));
        await logAuction(connection, escrowAccount.publicKey, program)
        assert.equal(await getTokenBalance(bidderFtTempAccountKeypair.publicKey, connection), price)
        assert.equal(await getTokenBalance(bidderFtPubkey, connection), 500 - price)
    }

    it("Bidder1", async () => {
        await bidder(initialPrice + 1, ftMintPubkey, bidder1Account, bidder1FtTokenAccountPubkey)
    })

    it("Bidder2", async () => {
        await bidder(initialPrice + 2, ftMintPubkey, bidder2Account, bidder2FtTokenAccountPubkey)
        assert.equal(await getTokenBalance(bidder1FtTokenAccountPubkey, connection), 500)
    })

    it("Receive", async () => {
        await new Promise((resolve) => setTimeout(resolve, (duration - 3) * 1000));
        const auction = await program.account.auction.fetch(escrowAccount.publicKey)
        const pda = await PublicKey.findProgramAddress([Buffer.from("escrow")], program.programId);
        const winningBidderNftReceivingAccount = anchor.web3.Keypair.generate();
        const signature = await program.rpc.close(
            {
                accounts: {
                    winningBidder: auction.highestBidderPubkey,
                    exhibitor: auction.exhibitorPubkey,
                    exhibitorNftTempAccount: auction.exhibitingNftTempPubkey,
                    exhibitorFtReceivingAccount: auction.exhibitorFtReceivingPubkey,
                    highestBidderFtTempAccount: auction.highestBidderFtTempPubkey,
                    highestBidderNftReceivingAccount: winningBidderNftReceivingAccount.publicKey,
                    escrowAccount: escrowAccount.publicKey,
                    clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
                    pda: pda[0],
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                instructions: [
                    ...await accountInstructions(connection, nftMintPubkey, winningBidderNftReceivingAccount.publicKey, bidder2Account.publicKey),
                ],
                signers: [bidder2Account, winningBidderNftReceivingAccount]
            }
        );
        console.log(`receive tx = ${signature}`)

        await new Promise((resolve) => setTimeout(resolve, 1500));

        const data = {
            NFT: {
                exhibitor: await getTokenBalance(exhibitorNftTokenAccountPubkey, connection),
                "Exhibitor Token Account": exhibitorNftTokenAccountPubkey.toBase58(),
                bidder2: await getTokenBalance(winningBidderNftReceivingAccount.publicKey, connection),
                "Bidder2 Token Account": winningBidderNftReceivingAccount.publicKey.toBase58(),
            },
            FT: {
                exhibitor: await getTokenBalance(exhibitorFtTokenAccountPubkey, connection),
                "Exhibitor Token Account": exhibitorFtTokenAccountPubkey.toBase58(),
                bidder2: await getTokenBalance(bidder2FtTokenAccountPubkey, connection),
                "Bidder2 Token Account": bidder2FtTokenAccountPubkey.toBase58(),
            },
        };
        console.table(data);

        assert.equal(await getTokenBalance(exhibitorNftTokenAccountPubkey, connection), 0)
        assert.equal(await getTokenBalance(winningBidderNftReceivingAccount.publicKey, connection), 1)
        assert.equal(await getTokenBalance(exhibitorFtTokenAccountPubkey, connection), 500 + 202)
        assert.equal(await getTokenBalance(bidder1FtTokenAccountPubkey, connection), 500)
        assert.equal(await getTokenBalance(bidder2FtTokenAccountPubkey, connection), 500 - 202)
        assert.ok(isNaN(await getTokenBalance(auction.highestBidderFtTempPubkey, connection)))
        assert.ok(isNaN(await getTokenBalance(auction.exhibitingNftTempPubkey, connection)))
    })

})

async function accountInstructions(connection: Connection, mintPubkey: PublicKey, taPubkey: PublicKey, creatorPubkey: PublicKey) {
    const createAccount = SystemProgram.createAccount({
        space: AccountLayout.span,
        lamports: await connection.getMinimumBalanceForRentExemption(
            AccountLayout.span
        ),
        fromPubkey: creatorPubkey,
        newAccountPubkey: taPubkey,
        programId: TOKEN_PROGRAM_ID,
    });
    const initAccount = createInitializeAccountInstruction(
        taPubkey,
        mintPubkey,
        creatorPubkey,
        TOKEN_PROGRAM_ID
    );
    return [createAccount, initAccount]
}

const getTokenBalance = async (
    pubkey: PublicKey,
    connection: Connection
) => {
    try {
        return parseInt(
            (await connection.getTokenAccountBalance(pubkey)).value.amount
        );
    } catch (e) {
        console.error(`Not a token account ${pubkey}`);
        return NaN;
    }
};

async function logAuction(connection: Connection, escrowPubkey: PublicKey, program : Program<AnchorAuction>) {
    const auction = await program.account.auction.fetch(escrowPubkey)
    console.table({
        exhibitorPubkey: auction.exhibitorPubkey.toBase58(),
        exhibitingNftTempPubkey: auction.exhibitingNftTempPubkey.toBase58(),
        exhibitorFtReceivingPubkey: auction.exhibitorFtReceivingPubkey.toBase58(),
        price: new anchor.BN(auction.price, 10, "le").toNumber(),
        endAt: new Date(
            new anchor.BN(auction.endAt, 10, "le").toNumber() * 1000
        ).toISOString(),
        highestBidderPubkey: auction.highestBidderPubkey.toBase58(),
        highestBidderFtTempPubkey: auction.highestBidderFtTempPubkey.toBase58(),
        highestBidderFtReturningPubkey: auction.highestBidderFtReturningPubkey.toBase58(),
    });
}
