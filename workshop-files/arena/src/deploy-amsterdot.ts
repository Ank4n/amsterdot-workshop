import { ContractFactory } from "ethers";
import { createTestPairs } from "@polkadot/keyring/testingPairs";

import Simple from "../build/Simple.json";
import Advanced from "../build/Advanced.json";
import Random from "../build/Random.json";
import { setup } from "../utils/setup";

const main = async () => {
  const { wallet, provider } = await setup();
  const api = provider.api;

  console.log("Deploying");

  const simpleFactory = ContractFactory.fromSolidity(Simple).connect(wallet);
  const advancedFactory =
    ContractFactory.fromSolidity(Advanced).connect(wallet);
  const randomFactory =
    ContractFactory.fromSolidity(Random).connect(wallet);

  const instance0 = await simpleFactory.deploy(0);
  const instance1 = await simpleFactory.deploy(1);
  const instance2 = await simpleFactory.deploy(2);
  const instanceAdvanced = await advancedFactory.deploy();
  const instanceRandom = await randomFactory.deploy();

  console.log("Simple:", [
    instance0.address,
    instance1.address,
    instance2.address,
  ]);
  console.log("Advanced", instanceAdvanced.address);
  console.log("Random", instanceRandom.address);

  const alice = createTestPairs().alice;

  const sendTx = (tx: ReturnType<typeof api.tx.system.remark>, nolog = false) =>
    new Promise<void>((resolve, reject) => {
      tx.signAndSend(alice, (res) => {
        if (res.isInBlock || res.isFinalized) {
          resolve();
        }
        if (res.isError) {
          reject();
        }
        if (res.events.length && !nolog) {
          console.dir(
            res.events.map((e) => e.event.toHuman()),
            { depth: 5 }
          );
        }
      });
    });

  await sendTx(
    api.tx.utility.batch([
      api.tx.arena.register(instance0.address),
      api.tx.arena.register(instance1.address),
      api.tx.arena.register(instance2.address),
      api.tx.arena.register(instanceAdvanced.address),
      api.tx.arena.register(instanceRandom.address),
    ])
  );

  await api.disconnect();
};

main();
