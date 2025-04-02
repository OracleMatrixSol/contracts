const anchor = require("@coral-xyz/anchor");
const { SystemProgram } = anchor.web3;

describe("oraclematrix", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Oraclematrix;

  it("Initializes the config", async () => {
    const config = anchor.web3.Keypair.generate();
    await program.methods
      .initialize()
      .accounts({
        config: config.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([config])
      .rpc();
    const configAccount = await program.account.config.fetch(config.publicKey);
    assert.equal(configAccount.authority.toBase58(), provider.wallet.publicKey.toBase58());
  });

  it("Registers a data source", async () => {
    const config = anchor.web3.Keypair.generate();
    const dataSource = anchor.web3.Keypair.generate();
    await program.methods
      .initialize()
      .accounts({
        config: config.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([config])
      .rpc();
    await program.methods
      .registerDataSource(new anchor.BN(100))
      .accounts({
        config: config.publicKey,
        dataSource: dataSource.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([dataSource])
      .rpc();
    const dataSourceAccount = await program.account.dataSource.fetch(dataSource.publicKey);
    assert.equal(dataSourceAccount.reputation.toNumber(), 100);
  });
});
